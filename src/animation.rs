use self::unsync::MutableAnimation;
use std::rc::{Rc, Weak};
use std::cell::{Cell, RefCell};
use futures::{Async, task};
use futures::future::Future;
use futures::task::Task;
use futures_signals::signal::{Signal, State, WaitFor};
use futures_signals::signal::unsync::MutableSignal;
use futures_signals::signal_vec::{SignalVec, VecChange};
use stdweb::Value;


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


struct TimestampsGlobal {
    raf: Option<Raf>,
    value: Option<f64>,
    // TODO make this more efficient
    states: Vec<Weak<TimestampsState>>,
}

#[derive(Clone, Copy)]
enum TimestampsEnum {
    First,
    Changed,
    NotChanged,
}

struct TimestampsState {
    state: Cell<TimestampsEnum>,
    task: RefCell<Option<Task>>,
    global: Rc<RefCell<TimestampsGlobal>>,
}

// TODO make this more efficient
pub struct Timestamps(Rc<TimestampsState>);

impl Signal for Timestamps {
    type Item = Option<f64>;

    fn poll(&mut self) -> State<Self::Item> {
        match self.0.state.get() {
            TimestampsEnum::Changed => {
                self.0.state.set(TimestampsEnum::NotChanged);
                // TODO make this more efficient ?
                State::Changed(self.0.global.borrow().value.clone())
            },
            TimestampsEnum::First => {
                self.0.state.set(TimestampsEnum::NotChanged);
                State::Changed(None)
            },
            TimestampsEnum::NotChanged => {
                *self.0.task.borrow_mut() = Some(task::current());
                State::NotChanged
            },
        }
    }
}

thread_local! {
    static TIMESTAMPS: Rc<RefCell<TimestampsGlobal>> = Rc::new(RefCell::new(TimestampsGlobal {
        raf: None,
        value: None,
        states: vec![],
    }));
}

pub fn timestamps() -> Timestamps {
    TIMESTAMPS.with(|timestamps| {
        let state = Rc::new(TimestampsState {
            state: Cell::new(TimestampsEnum::First),
            task: RefCell::new(None),
            global: timestamps.clone(),
        });

        {
            let mut lock = timestamps.borrow_mut();

            lock.states.push(Rc::downgrade(&state));

            if let None = lock.raf {
                let timestamps = timestamps.clone();

                lock.raf = Some(Raf::new(move |time| {
                    let mut lock = timestamps.borrow_mut();

                    lock.value = Some(time);

                    lock.states.retain(|state| {
                        if let Some(state) = state.upgrade() {
                            state.state.set(TimestampsEnum::Changed);

                            let mut lock = state.task.borrow_mut();

                            if let Some(task) = lock.take() {
                                drop(lock);
                                task.notify();
                            }

                            true

                        } else {
                            false
                        }
                    });

                    if lock.states.len() == 0 {
                        lock.raf = None;
                        lock.states = vec![];
                    }
                }));
            }
        }

        Timestamps(state)
    })
}


pub trait AnimatedSignalVec: SignalVec {
    type AnimatedSignal: Signal<Item = Percentage>;

    fn animated_map<A, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
        where F: FnMut(Self::Item, Self::AnimatedSignal) -> A,
              Self: Sized;
}

impl<S: SignalVec> AnimatedSignalVec for S {
    type AnimatedSignal = MutableSignal<Percentage>;

    #[inline]
    fn animated_map<A, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
        where F: FnMut(Self::Item, Self::AnimatedSignal) -> A {
        AnimatedMap {
            duration: duration,
            animations: vec![],
            signal: Some(self),
            callback: f,
        }
    }
}


struct AnimatedMapState {
    animation: MutableAnimation,
    removing: Option<WaitFor<MutableSignal<Percentage>>>,
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
          F: FnMut(S::Item, MutableSignal<Percentage>) -> A {

    fn animated_state(&self) -> AnimatedMapState {
        let state = AnimatedMapState {
            animation: MutableAnimation::new(self.duration),
            removing: None,
        };

        state.animation.animate_to(Percentage::new_unchecked(1.0));

        state
    }

    fn remove_index(&mut self, index: usize) -> Async<Option<VecChange<A>>> {
        if index == (self.animations.len() - 1) {
            self.animations.pop();
            Async::Ready(Some(VecChange::Pop {}))

        } else {
            self.animations.remove(index);
            Async::Ready(Some(VecChange::RemoveAt { index }))
        }
    }

    fn should_remove(&mut self, index: usize) -> bool {
        let state = &mut self.animations[index];

        state.animation.animate_to(Percentage::new_unchecked(0.0));

        let mut future = state.animation.signal().wait_for(Percentage::new_unchecked(0.0));

        if future.poll().unwrap().is_ready() {
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
          F: FnMut(S::Item, MutableSignal<Percentage>) -> A {
    type Item = A;

    // TODO this can probably be implemented more efficiently
    fn poll(&mut self) -> Async<Option<VecChange<Self::Item>>> {
        let mut is_done = true;

        // TODO is this loop correct ?
        while let Some(mut signal) = self.signal.take() {
            match signal.poll() {
                Async::Ready(Some(change)) => {
                    self.signal = Some(signal);

                    return match change {
                        // TODO maybe it should play remove / insert animations for this ?
                        VecChange::Replace { values } => {
                            self.animations = Vec::with_capacity(values.len());

                            Async::Ready(Some(VecChange::Replace {
                                values: values.into_iter().map(|value| {
                                    let state = AnimatedMapState {
                                        animation: MutableAnimation::new_with_initial(self.duration, Percentage::new_unchecked(1.0)),
                                        removing: None,
                                    };

                                    let value = (self.callback)(value, state.animation.signal());

                                    self.animations.push(state);

                                    value
                                }).collect()
                            }))
                        },

                        VecChange::InsertAt { index, value } => {
                            let index = self.find_index(index).unwrap_or_else(|| self.animations.len());
                            let state = self.animated_state();
                            let value = (self.callback)(value, state.animation.signal());
                            self.animations.insert(index, state);
                            Async::Ready(Some(VecChange::InsertAt { index, value }))
                        },

                        VecChange::Push { value } => {
                            let state = self.animated_state();
                            let value = (self.callback)(value, state.animation.signal());
                            self.animations.push(state);
                            Async::Ready(Some(VecChange::Push { value }))
                        },

                        VecChange::UpdateAt { index, value } => {
                            let index = self.find_index(index).expect("Could not find value");
                            let state = &self.animations[index];
                            let value = (self.callback)(value, state.animation.signal());
                            Async::Ready(Some(VecChange::UpdateAt { index, value }))
                        },

                        VecChange::RemoveAt { index } => {
                            let index = self.find_index(index).expect("Could not find value");

                            if self.should_remove(index) {
                                self.remove_index(index)

                            } else {
                                continue;
                            }
                        },

                        VecChange::Pop {} => {
                            let index = self.find_last_index().expect("Cannot pop from empty vec");

                            if self.should_remove(index) {
                                self.remove_index(index)

                            } else {
                                continue;
                            }
                        },

                        // TODO maybe it should play remove animation for this ?
                        VecChange::Clear {} => {
                            self.animations = vec![];
                            Async::Ready(Some(VecChange::Clear {}))
                        },
                    }
                },
                Async::Ready(None) => {
                    break;
                },
                Async::NotReady => {
                    self.signal = Some(signal);
                    is_done = false;
                    break;
                },
            }
        }

        let mut is_removing = false;

        // TODO make this more efficient (e.g. using a similar strategy as FuturesUnordered)
        // This uses rposition so that way it will return VecChange::Pop in more situations
        let index = self.animations.iter_mut().rposition(|state| {
            if let Some(ref mut future) = state.removing {
                is_removing = true;
                future.poll().unwrap().is_ready()

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
                Async::NotReady
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
}

impl Into<f64> for Percentage {
    #[inline]
    fn into(self) -> f64 {
        self.0
    }
}

#[inline]
fn range_inclusive(percentage: f64, low: f64, high: f64) -> f64 {
    low + (percentage * (high - low))
}


pub mod unsync {
    use super::{Percentage, range_inclusive, timestamps};
    use operations::spawn_future;
    use std::rc::Rc;
    use std::cell::{Cell, RefCell};
    use futures_signals::signal::{Signal, CancelableFutureHandle};
    use futures_signals::signal::unsync::{Mutable, MutableSignal};
    use discard::DiscardOnDrop;


    struct MutableAnimationState {
        playing: Cell<bool>,
        duration: Cell<f64>,
        value: Mutable<Percentage>,
        end: Cell<Percentage>,
        animating: RefCell<Option<DiscardOnDrop<CancelableFutureHandle>>>,
    }

    #[derive(Clone)]
    pub struct MutableAnimation(Rc<MutableAnimationState>);

    impl MutableAnimation {
        #[inline]
        pub fn new_with_initial(duration: f64, initial: Percentage) -> Self {
            debug_assert!(duration >= 0.0);

            MutableAnimation(Rc::new(MutableAnimationState {
                playing: Cell::new(true),
                duration: Cell::new(duration),
                value: Mutable::new(initial),
                end: Cell::new(initial),
                animating: RefCell::new(None),
            }))
        }

        #[inline]
        pub fn new(duration: f64) -> Self {
            Self::new_with_initial(duration, Percentage::new_unchecked(0.0))
        }

        #[inline]
        fn stop_animating(&self) {
            *self.0.animating.borrow_mut() = None;
        }

        fn start_animating(&self) {
            if self.0.playing.get() {
                // TODO use Copy constraint to make value.get() faster ?
                let start: f64 = self.0.value.get().into();
                let end: f64 = self.0.end.get().into();

                if start != end {
                    let duration = self.0.duration.get();

                    if duration > 0.0 {
                        let duration = (end - start).abs() * duration;

                        let state = self.clone();

                        let mut starting_time = None;

                        *self.0.animating.borrow_mut() = Some(spawn_future(
                            timestamps()
                                .map(move |current_time| {
                                    if let Some(current_time) = current_time {
                                        let starting_time = *starting_time.get_or_insert(current_time);

                                        let diff = (current_time - starting_time) / duration;

                                        // TODO don't update if the new value is the same as the old value
                                        if diff >= 1.0 {
                                            state.stop_animating();
                                            state.0.value.set(Percentage::new_unchecked(end));
                                            true

                                        } else {
                                            state.0.value.set(Percentage::new_unchecked(range_inclusive(diff, start, end)));
                                            false
                                        }

                                    } else {
                                        false
                                    }
                                })
                                .wait_for(true)
                        ));

                    } else {
                        self.stop_animating();
                        self.0.value.set(Percentage::new_unchecked(end));
                    }

                } else {
                    // TODO is this necessary ?
                    self.stop_animating();
                }
            }
        }

        pub fn set_duration(&self, duration: f64) {
            debug_assert!(duration >= 0.0);

            if self.0.duration.get() != duration {
                self.0.duration.set(duration);
                self.start_animating();
            }
        }

        #[inline]
        pub fn pause(&self) {
            self.0.playing.set(false);
            self.stop_animating();
        }

        #[inline]
        pub fn play(&self) {
            self.0.playing.set(true);
            self.start_animating();
        }

        pub fn jump_to(&self, end: Percentage) {
            self.stop_animating();

            self.0.end.set(end);

            // TODO use Copy constraint to make value.get() faster ?
            if self.0.value.get() != end {
                self.0.value.set(end);
            }
        }

        pub fn animate_to(&self, end: Percentage) {
            if self.0.end.get() != end {
                if self.0.duration.get() <= 0.0 {
                    self.jump_to(end);

                } else {
                    self.0.end.set(end);
                    self.start_animating();
                }
            }
        }

        #[inline]
        pub fn signal(&self) -> MutableSignal<Percentage> {
            self.0.value.signal()
        }
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
