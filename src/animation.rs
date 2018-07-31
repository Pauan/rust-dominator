use std::fmt;
use std::sync::{Arc, Weak, Mutex, RwLock};
use futures_core::Async;
use futures_core::future::Future;
use futures_core::task::{Context, Waker};
use futures_signals::CancelableFutureHandle;
use futures_signals::signal::{Signal, SignalExt, WaitFor, MutableSignal, Mutable};
use futures_signals::signal_vec::{SignalVec, VecDiff};
use stdweb::Value;
use discard::DiscardOnDrop;
use operations::spawn_future;


// TODO generalize this so it works for any target, not just JS
struct Raf(Value);

impl Raf {
    #[inline]
    fn new<F>(callback: F) -> Self where F: FnMut(f64) + 'static {
        Raf(js!(
            var callback = @{callback};

            function loop(time) {
                value.id = requestAnimationFrame(loop);
                callback(time);
            }

            var value = {
                callback: callback,
                id: requestAnimationFrame(loop)
            };

            return value;
        ))
    }
}

impl Drop for Raf {
    #[inline]
    fn drop(&mut self) {
        js! { @(no_return)
            var self = @{&self.0};
            cancelAnimationFrame(self.id);
            self.callback.drop();
        }
    }
}


struct TimestampsInner {
    raf: Option<Raf>,
    // TODO make this more efficient
    states: Vec<Weak<Mutex<TimestampsState>>>,
}

struct TimestampsGlobal {
    inner: Mutex<TimestampsInner>,
    value: Arc<RwLock<Option<f64>>>,
}

enum TimestampsEnum {
    First,
    Changed,
    NotChanged,
}

struct TimestampsState {
    state: TimestampsEnum,
    waker: Option<Waker>,
}

pub struct Timestamps {
    state: Arc<Mutex<TimestampsState>>,
    // TODO verify that there aren't any Arc cycles
    value: Arc<RwLock<Option<f64>>>,
}

impl Signal for Timestamps {
    type Item = Option<f64>;

    // TODO implement Async::Ready(None)
    fn poll_change(&mut self, cx: &mut Context) -> Async<Option<Self::Item>> {
        let mut lock = self.state.lock().unwrap();

        match lock.state {
            TimestampsEnum::Changed => {
                lock.state = TimestampsEnum::NotChanged;
                Async::Ready(Some(*self.value.read().unwrap()))
            },
            TimestampsEnum::First => {
                lock.state = TimestampsEnum::NotChanged;
                Async::Ready(Some(None))
            },
            TimestampsEnum::NotChanged => {
                lock.waker = Some(cx.waker().clone());
                Async::Pending
            },
        }
    }
}

lazy_static! {
    static ref TIMESTAMPS: Arc<TimestampsGlobal> = Arc::new(TimestampsGlobal {
        inner: Mutex::new(TimestampsInner {
            raf: None,
            states: vec![],
        }),
        value: Arc::new(RwLock::new(None)),
    });
}

pub fn timestamps() -> Timestamps {
    let timestamps = Timestamps {
        state: Arc::new(Mutex::new(TimestampsState {
            state: TimestampsEnum::First,
            waker: None,
        })),
        value: TIMESTAMPS.value.clone(),
    };

    {
        let mut lock = TIMESTAMPS.inner.lock().unwrap();

        lock.states.push(Arc::downgrade(&timestamps.state));

        if let None = lock.raf {
            let global = TIMESTAMPS.clone();

            lock.raf = Some(Raf::new(move |time| {
                let mut lock = global.inner.lock().unwrap();
                let mut value = global.value.write().unwrap();

                *value = Some(time);

                lock.states.retain(|state| {
                    if let Some(state) = state.upgrade() {
                        let mut lock = state.lock().unwrap();

                        lock.state = TimestampsEnum::Changed;

                        if let Some(waker) = lock.waker.take() {
                            drop(lock);
                            waker.wake();
                        }

                        true

                    } else {
                        false
                    }
                });

                if lock.states.len() == 0 {
                    lock.raf = None;
                    // TODO is this a good idea ?
                    lock.states = vec![];
                }
            }));
        }
    }

    timestamps
}


pub trait AnimatedSignalVec: SignalVec {
    type Animation;

    fn animated_map<A, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
        where F: FnMut(Self::Item, Self::Animation) -> A,
              Self: Sized;
}

impl<S: SignalVec> AnimatedSignalVec for S {
    type Animation = AnimatedMapBroadcaster;

    #[inline]
    fn animated_map<A, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
        where F: FnMut(Self::Item, Self::Animation) -> A {
        AnimatedMap {
            duration: duration,
            animations: vec![],
            signal: Some(self),
            callback: f,
        }
    }
}


pub struct AnimatedMapBroadcaster(MutableAnimation);

impl AnimatedMapBroadcaster {
    // TODO it should return a custom type
    #[inline]
    pub fn signal(&self) -> MutableAnimationSignal {
        self.0.signal()
    }
}


struct AnimatedMapState {
    animation: MutableAnimation,
    removing: Option<WaitFor<MutableAnimationSignal>>,
}

// TODO move this into signals crate and also generalize it to work with any future, not just animations
pub struct AnimatedMap<A, B> {
    duration: f64,
    animations: Vec<AnimatedMapState>,
    signal: Option<A>,
    callback: B,
}

impl<A, F, S> AnimatedMap<S, F>
    where S: SignalVec,
          F: FnMut(S::Item, AnimatedMapBroadcaster) -> A {

    fn animated_state(&self) -> AnimatedMapState {
        let state = AnimatedMapState {
            animation: MutableAnimation::new(self.duration),
            removing: None,
        };

        state.animation.animate_to(Percentage::new_unchecked(1.0));

        state
    }

    fn remove_index(&mut self, index: usize) -> Async<Option<VecDiff<A>>> {
        if index == (self.animations.len() - 1) {
            self.animations.pop();
            Async::Ready(Some(VecDiff::Pop {}))

        } else {
            self.animations.remove(index);
            Async::Ready(Some(VecDiff::RemoveAt { index }))
        }
    }

    fn should_remove(&mut self, cx: &mut Context, index: usize) -> bool {
        let state = &mut self.animations[index];

        state.animation.animate_to(Percentage::new_unchecked(0.0));

        let mut future = state.animation.signal().wait_for(Percentage::new_unchecked(0.0));

        if future.poll(cx).unwrap().is_ready() {
            true

        } else {
            state.removing = Some(future);
            false
        }
    }

    fn find_index(&self, parent_index: usize) -> Option<usize> {
        let mut seen = 0;

        // TODO is there a combinator that can simplify this ?
        self.animations.iter().position(|state| {
            if state.removing.is_none() {
                if seen == parent_index {
                    true

                } else {
                    seen += 1;
                    false
                }

            } else {
                false
            }
        })
    }

    #[inline]
    fn find_last_index(&self) -> Option<usize> {
        self.animations.iter().rposition(|state| state.removing.is_none())
    }
}

impl<A, F, S> SignalVec for AnimatedMap<S, F>
    where S: SignalVec,
          F: FnMut(S::Item, AnimatedMapBroadcaster) -> A {
    type Item = A;

    // TODO this can probably be implemented more efficiently
    fn poll_vec_change(&mut self, cx: &mut Context) -> Async<Option<VecDiff<Self::Item>>> {
        let mut is_done = true;

        // TODO is this loop correct ?
        while let Some(result) = self.signal.as_mut().map(|signal| signal.poll_vec_change(cx)) {
            match result {
                Async::Ready(Some(change)) => return match change {
                    // TODO maybe it should play remove / insert animations for this ?
                    VecDiff::Replace { values } => {
                        self.animations = Vec::with_capacity(values.len());

                        Async::Ready(Some(VecDiff::Replace {
                            values: values.into_iter().map(|value| {
                                let state = AnimatedMapState {
                                    animation: MutableAnimation::new_with_initial(self.duration, Percentage::new_unchecked(1.0)),
                                    removing: None,
                                };

                                let value = (self.callback)(value, AnimatedMapBroadcaster(state.animation.raw_clone()));

                                self.animations.push(state);

                                value
                            }).collect()
                        }))
                    },

                    VecDiff::InsertAt { index, value } => {
                        let index = self.find_index(index).unwrap_or_else(|| self.animations.len());
                        let state = self.animated_state();
                        let value = (self.callback)(value, AnimatedMapBroadcaster(state.animation.raw_clone()));
                        self.animations.insert(index, state);
                        Async::Ready(Some(VecDiff::InsertAt { index, value }))
                    },

                    VecDiff::Push { value } => {
                        let state = self.animated_state();
                        let value = (self.callback)(value, AnimatedMapBroadcaster(state.animation.raw_clone()));
                        self.animations.push(state);
                        Async::Ready(Some(VecDiff::Push { value }))
                    },

                    VecDiff::UpdateAt { index, value } => {
                        let index = self.find_index(index).expect("Could not find value");
                        let state = &self.animations[index];
                        let value = (self.callback)(value, AnimatedMapBroadcaster(state.animation.raw_clone()));
                        Async::Ready(Some(VecDiff::UpdateAt { index, value }))
                    },

                    // TODO test this
                    // TODO should this be treated as a removal + insertion ?
                    VecDiff::Move { old_index, new_index } => {
                        let old_index = self.find_index(old_index).expect("Could not find value");

                        let state = self.animations.remove(old_index);

                        let new_index = self.find_index(new_index).unwrap_or_else(|| self.animations.len());

                        self.animations.insert(new_index, state);

                        Async::Ready(Some(VecDiff::Move { old_index, new_index }))
                    },

                    VecDiff::RemoveAt { index } => {
                        let index = self.find_index(index).expect("Could not find value");

                        if self.should_remove(cx, index) {
                            self.remove_index(index)

                        } else {
                            continue;
                        }
                    },

                    VecDiff::Pop {} => {
                        let index = self.find_last_index().expect("Cannot pop from empty vec");

                        if self.should_remove(cx, index) {
                            self.remove_index(index)

                        } else {
                            continue;
                        }
                    },

                    // TODO maybe it should play remove animation for this ?
                    VecDiff::Clear {} => {
                        self.animations.clear();
                        Async::Ready(Some(VecDiff::Clear {}))
                    },
                },
                Async::Ready(None) => {
                    self.signal = None;
                    break;
                },
                Async::Pending => {
                    is_done = false;
                    break;
                },
            }
        }

        let mut is_removing = false;

        // TODO make this more efficient (e.g. using a similar strategy as FuturesUnordered)
        // This uses rposition so that way it will return VecDiff::Pop in more situations
        let index = self.animations.iter_mut().rposition(|state| {
            if let Some(ref mut future) = state.removing {
                is_removing = true;
                future.poll(cx).unwrap().is_ready()

            } else {
                false
            }
        });

        match index {
            Some(index) => {
                self.remove_index(index)
            },
            None => if is_done && !is_removing {
                Async::Ready(None)

            } else {
                Async::Pending
            },
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Percentage(f64);

impl Percentage {
    #[inline]
    pub fn new(input: f64) -> Self {
        debug_assert!(input >= 0.0 && input <= 1.0);
        Self::new_unchecked(input)
    }

    #[inline]
    pub fn new_unchecked(input: f64) -> Self {
        Percentage(input)
    }

    #[inline]
    pub fn map<F>(self, f: F) -> Self where F: FnOnce(f64) -> f64 {
        Self::new(f(self.0))
    }

    #[inline]
    pub fn map_unchecked<F>(self, f: F) -> Self where F: FnOnce(f64) -> f64 {
        Self::new_unchecked(f(self.0))
    }

    #[inline]
    pub fn invert(self) -> Self {
        // TODO use new instead ?
        Self::new_unchecked(1.0 - self.0)
    }

    #[inline]
    pub fn range_inclusive(&self, low: f64, high: f64) -> f64 {
        range_inclusive(self.0, low, high)
    }

    // TODO figure out better name
    #[inline]
    pub fn into_f64(self) -> f64 {
        self.0
    }

    pub fn none_if(self, percentage: f64) -> Option<Self> {
        if self.0 == percentage {
            None

        } else {
            Some(self)
        }
    }
}

#[inline]
fn range_inclusive(percentage: f64, low: f64, high: f64) -> f64 {
    low + (percentage * (high - low))
}


/*pub struct MutableTimestamps<F> {
    callback: Arc<F>,
    animating: Mutex<Option<DiscardOnDrop<CancelableFutureHandle>>>,
}

impl MutableTimestamps<F> where F: FnMut(f64) {
    pub fn new(callback: F) -> Self {
        Self {
            callback: Arc::new(callback),
            animating: Mutex::new(None),
        }
    }

    pub fn stop(&self) {
        let mut lock = self.animating.lock().unwrap();
        *lock = None;
    }

    pub fn start(&self) {
        let mut lock = self.animating.lock().unwrap();

        if let None = animating {
            let callback = self.callback.clone();

            let mut starting_time = None;

            *animating = Some(OnTimestampDiff::new(move |value| callback(value)));
        }
    }
}*/


pub struct OnTimestampDiff(DiscardOnDrop<CancelableFutureHandle>);

impl OnTimestampDiff {
    pub fn new<F>(mut callback: F) -> Self where F: FnMut(f64) + 'static {
        let mut starting_time = None;

        OnTimestampDiff(spawn_future(
            timestamps()
                .for_each(move |current_time| {
                    if let Some(current_time) = current_time {
                        let starting_time = *starting_time.get_or_insert(current_time);

                        callback(current_time - starting_time);
                    }

                    Ok(())
                })
        ))
    }
}

impl fmt::Debug for OnTimestampDiff {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("OnTimestampDiff")
            .finish()
    }
}


pub struct MutableAnimationSignal(MutableSignal<Percentage>);

impl Signal for MutableAnimationSignal {
    type Item = Percentage;

    #[inline]
    fn poll_change(&mut self, cx: &mut Context) -> Async<Option<Self::Item>> {
        self.0.poll_change(cx)
    }
}


struct MutableAnimationState {
    playing: bool,
    duration: f64,
    end: Percentage,
    animating: Option<OnTimestampDiff>,
}

struct MutableAnimationInner {
    state: Mutex<MutableAnimationState>,
    value: Mutable<Percentage>,
}

pub struct MutableAnimation {
    inner: Arc<MutableAnimationInner>,
}

impl fmt::Debug for MutableAnimation {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let state = self.inner.state.lock().unwrap();

        fmt.debug_struct("MutableAnimation")
            .field("playing", &state.playing)
            .field("duration", &state.duration)
            .field("current", &self.inner.value.get())
            .field("end", &state.end)
            .finish()
    }
}

impl MutableAnimation {
    #[inline]
    pub fn new_with_initial(duration: f64, initial: Percentage) -> Self {
        debug_assert!(duration >= 0.0);

        Self {
            inner: Arc::new(MutableAnimationInner {
                state: Mutex::new(MutableAnimationState {
                    playing: true,
                    duration: duration,
                    end: initial,
                    animating: None,
                }),
                value: Mutable::new(initial),
            }),
        }
    }

    #[inline]
    pub fn new(duration: f64) -> Self {
        Self::new_with_initial(duration, Percentage::new_unchecked(0.0))
    }

    #[inline]
    fn raw_clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    #[inline]
    fn stop_animating(lock: &mut MutableAnimationState) {
        lock.animating = None;
    }

    fn start_animating(&self, lock: &mut MutableAnimationState) {
        if lock.playing {
            // TODO use Copy constraint to make value.get() faster ?
            let start: f64 = self.inner.value.get().into_f64();
            let end: f64 = lock.end.into_f64();

            if start != end {
                if lock.duration > 0.0 {
                    let duration = (end - start).abs() * lock.duration;

                    let state = self.raw_clone();

                    lock.animating = Some(OnTimestampDiff::new(move |diff| {
                        let diff = diff / duration;

                        // TODO test the performance of set_neq
                        if diff >= 1.0 {
                            {
                                let mut lock = state.inner.state.lock().unwrap();
                                Self::stop_animating(&mut lock);
                            }
                            state.inner.value.set_neq(Percentage::new_unchecked(end));

                        } else {
                            state.inner.value.set_neq(Percentage::new_unchecked(range_inclusive(diff, start, end)));
                        }
                    }));

                } else {
                    Self::stop_animating(lock);
                    self.inner.value.set_neq(Percentage::new_unchecked(end));
                }

            } else {
                // TODO is this necessary ?
                Self::stop_animating(lock);
            }
        }
    }

    pub fn set_duration(&self, duration: f64) {
        debug_assert!(duration >= 0.0);

        let mut lock = self.inner.state.lock().unwrap();

        if lock.duration != duration {
            lock.duration = duration;
            self.start_animating(&mut lock);
        }
    }

    #[inline]
    pub fn pause(&self) {
        let mut lock = self.inner.state.lock().unwrap();

        if lock.playing {
            lock.playing = false;
            Self::stop_animating(&mut lock);
        }
    }

    #[inline]
    pub fn play(&self) {
        let mut lock = self.inner.state.lock().unwrap();

        if !lock.playing {
            lock.playing = true;
            self.start_animating(&mut lock);
        }
    }

    fn _jump_to(mut lock: &mut MutableAnimationState, mutable: &Mutable<Percentage>, end: Percentage) {
        Self::stop_animating(&mut lock);

        lock.end = end;

        mutable.set_neq(end);
    }

    pub fn jump_to(&self, end: Percentage) {
        let mut lock = self.inner.state.lock().unwrap();

        Self::_jump_to(&mut lock, &self.inner.value, end);
    }

    pub fn animate_to(&self, end: Percentage) {
        let mut lock = self.inner.state.lock().unwrap();

        if lock.end != end {
            if lock.duration <= 0.0 {
                Self::_jump_to(&mut lock, &self.inner.value, end);

            } else {
                lock.end = end;
                self.start_animating(&mut lock);
            }
        }
    }

    #[inline]
    pub fn signal(&self) -> MutableAnimationSignal {
        MutableAnimationSignal(self.inner.value.signal())
    }

    #[inline]
    pub fn current_percentage(&self) -> Percentage {
        self.inner.value.get()
    }
}


pub mod easing {
    use super::Percentage;

    // TODO should this use map rather than map_unchecked ?
    #[inline]
    pub fn powi(p: Percentage, n: i32) -> Percentage {
        p.map_unchecked(|p| p.powi(n))
    }

    #[inline]
    pub fn cubic(p: Percentage) -> Percentage {
        powi(p, 3)
    }

    #[inline]
    pub fn out<F>(p: Percentage, f: F) -> Percentage where F: FnOnce(Percentage) -> Percentage {
        f(p.invert()).invert()
    }

    pub fn in_out<F>(p: Percentage, f: F) -> Percentage where F: FnOnce(Percentage) -> Percentage {
        p.map_unchecked(|p| {
            if p <= 0.5 {
                f(Percentage::new_unchecked(p * 2.0)).into_f64() / 2.0

            } else {
                1.0 - (f(Percentage::new_unchecked((1.0 - p) * 2.0)).into_f64() / 2.0)
            }
        })
    }
}
