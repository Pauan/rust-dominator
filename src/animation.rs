use std::cell::RefCell;
use std::fmt;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex, Weak};
use std::task::{Context, Poll, Waker};

use discard::DiscardOnDrop;
use futures_signals::signal::{Mutable, MutableSignal, Signal, SignalExt, WaitFor};
use futures_signals::signal_vec::{SignalVec, VecDiff};
use futures_signals::CancelableFutureHandle;
use futures_util::future::{ready, FutureExt};
use pin_project::pin_project;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, UnwrapThrowExt};
use web_sys::window;

use crate::operations::spawn_future;

struct RafState {
    id: i32,
    closure: Closure<dyn FnMut(f64)>,
}

// TODO generalize this so it works for any target, not just JS
// TODO move this into gloo
struct Raf {
    state: Rc<RefCell<Option<RafState>>>,
}

impl Raf {
    fn new<F>(mut callback: F) -> Self
    where
        F: FnMut(f64) + 'static,
    {
        let state: Rc<RefCell<Option<RafState>>> = Rc::new(RefCell::new(None));

        fn schedule(callback: &Closure<dyn FnMut(f64)>) -> i32 {
            window()
                .unwrap_throw()
                .request_animation_frame(callback.as_ref().unchecked_ref())
                .unwrap_throw()
        }

        let closure = {
            let state = state.clone();

            Closure::wrap(Box::new(move |time| {
                {
                    let mut state = state.borrow_mut();
                    let state = state.as_mut().unwrap_throw();
                    state.id = schedule(&state.closure);
                }

                callback(time);
            }) as Box<dyn FnMut(f64)>)
        };

        *state.borrow_mut() = Some(RafState {
            id: schedule(&closure),
            closure,
        });

        Self { state }
    }
}

impl Drop for Raf {
    fn drop(&mut self) {
        // The take is necessary in order to prevent an Rc leak
        let state = self.state.borrow_mut().take().unwrap_throw();

        window()
            .unwrap_throw()
            .cancel_animation_frame(state.id)
            .unwrap_throw();
    }
}

struct TimestampsManager {
    raf: Option<Raf>,
    // TODO make this more efficient
    states: Vec<Weak<Mutex<TimestampsState>>>,
}

impl TimestampsManager {
    fn new() -> Self {
        Self {
            raf: None,
            states: vec![],
        }
    }
}

#[derive(Debug)]
struct TimestampsState {
    changed: bool,
    value: Option<f64>,
    waker: Option<Waker>,
}

impl TimestampsState {
    fn new() -> Self {
        Self {
            changed: true,
            value: None,
            waker: None,
        }
    }
}

#[must_use = "Signals do nothing unless polled"]
#[derive(Debug)]
pub struct Timestamps {
    state: Arc<Mutex<TimestampsState>>,
}

impl Timestamps {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TimestampsState::new())),
        }
    }
}

impl Signal for Timestamps {
    type Item = Option<f64>;

    // TODO implement Poll::Ready(None)
    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let mut lock = self.state.lock().unwrap_throw();

        if lock.changed {
            lock.changed = false;
            Poll::Ready(Some(lock.value))
        } else {
            lock.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

// TODO somehow share this safely between threads ?
thread_local! {
    static TIMESTAMPS_MANAGER: Rc<RefCell<TimestampsManager>> = Rc::new(RefCell::new(TimestampsManager::new()));
}

pub fn timestamps() -> Timestamps {
    TIMESTAMPS_MANAGER.with(|timestamps_manager| {
        let timestamps = Timestamps::new();

        {
            let mut lock = timestamps_manager.borrow_mut();

            lock.states.push(Arc::downgrade(&timestamps.state));

            if let None = lock.raf {
                let timestamps_manager = timestamps_manager.clone();

                lock.raf = Some(Raf::new(move |time| {
                    let mut lock = timestamps_manager.borrow_mut();

                    lock.states.retain(|state| {
                        if let Some(state) = state.upgrade() {
                            let mut lock = state.lock().unwrap_throw();

                            lock.changed = true;
                            lock.value = Some(time);

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
    })
}

pub trait AnimatedSignalVec: SignalVec {
    type Animation;

    fn animated_map<A, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
    where
        F: FnMut(Self::Item, Self::Animation) -> A,
        Self: Sized;
}

impl<S: SignalVec> AnimatedSignalVec for S {
    type Animation = AnimatedMapBroadcaster;

    #[inline]
    fn animated_map<A, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
    where
        F: FnMut(Self::Item, Self::Animation) -> A,
    {
        AnimatedMap {
            duration: duration,
            animations: vec![],
            signal: Some(self),
            callback: f,
        }
    }
}

#[derive(Debug)]
pub struct AnimatedMapBroadcaster(MutableAnimation);

impl AnimatedMapBroadcaster {
    // TODO it should return a custom type
    #[inline]
    pub fn signal(&self) -> MutableAnimationSignal {
        self.0.signal()
    }
}

#[derive(Debug)]
struct AnimatedMapState {
    animation: MutableAnimation,
    removing: Option<WaitFor<MutableAnimationSignal>>,
}

// TODO move this into signals crate and also generalize it to work with any future, not just animations
#[pin_project(project = AnimatedMapProj)]
#[derive(Debug)]
pub struct AnimatedMap<A, B> {
    duration: f64,
    animations: Vec<AnimatedMapState>,
    #[pin]
    signal: Option<A>,
    callback: B,
}

impl<A, F, S> AnimatedMap<S, F>
where
    S: SignalVec,
    F: FnMut(S::Item, AnimatedMapBroadcaster) -> A,
{
    fn animated_state(duration: f64) -> AnimatedMapState {
        let state = AnimatedMapState {
            animation: MutableAnimation::new(duration),
            removing: None,
        };

        state.animation.animate_to(Percentage::new_unchecked(1.0));

        state
    }

    fn remove_index(
        animations: &mut Vec<AnimatedMapState>,
        index: usize,
    ) -> Poll<Option<VecDiff<A>>> {
        if index == (animations.len() - 1) {
            animations.pop();
            Poll::Ready(Some(VecDiff::Pop {}))
        } else {
            animations.remove(index);
            Poll::Ready(Some(VecDiff::RemoveAt { index }))
        }
    }

    fn should_remove(
        animations: &mut Vec<AnimatedMapState>,
        cx: &mut Context,
        index: usize,
    ) -> bool {
        let state = &mut animations[index];

        state.animation.animate_to(Percentage::new_unchecked(0.0));

        let mut future = state
            .animation
            .signal()
            .wait_for(Percentage::new_unchecked(0.0));

        if future.poll_unpin(cx).is_ready() {
            true
        } else {
            state.removing = Some(future);
            false
        }
    }

    fn find_index(animations: &Vec<AnimatedMapState>, parent_index: usize) -> Option<usize> {
        let mut seen = 0;

        // TODO is there a combinator that can simplify this ?
        animations.iter().position(|state| {
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
    fn find_last_index(animations: &Vec<AnimatedMapState>) -> Option<usize> {
        animations
            .iter()
            .rposition(|state| state.removing.is_none())
    }
}

impl<A, F, S> SignalVec for AnimatedMap<S, F>
where
    S: SignalVec,
    F: FnMut(S::Item, AnimatedMapBroadcaster) -> A,
{
    type Item = A;

    // TODO this can probably be implemented more efficiently
    fn poll_vec_change(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<VecDiff<Self::Item>>> {
        let mut is_done = true;

        let AnimatedMapProj {
            mut animations,
            mut signal,
            callback,
            duration,
            ..
        } = self.project();

        // TODO is this loop correct ?
        while let Some(result) = signal
            .as_mut()
            .as_pin_mut()
            .map(|signal| signal.poll_vec_change(cx))
        {
            match result {
                Poll::Ready(Some(change)) => {
                    return match change {
                        // TODO maybe it should play remove / insert animations for this ?
                        VecDiff::Replace { values } => {
                            *animations = Vec::with_capacity(values.len());

                            Poll::Ready(Some(VecDiff::Replace {
                                values: values
                                    .into_iter()
                                    .map(|value| {
                                        let state = AnimatedMapState {
                                            animation: MutableAnimation::new_with_initial(
                                                *duration,
                                                Percentage::new_unchecked(1.0),
                                            ),
                                            removing: None,
                                        };

                                        let value = callback(
                                            value,
                                            AnimatedMapBroadcaster(state.animation.raw_clone()),
                                        );

                                        animations.push(state);

                                        value
                                    })
                                    .collect(),
                            }))
                        }

                        VecDiff::InsertAt { index, value } => {
                            let index = Self::find_index(&animations, index)
                                .unwrap_or_else(|| animations.len());
                            let state = Self::animated_state(*duration);
                            let value = callback(
                                value,
                                AnimatedMapBroadcaster(state.animation.raw_clone()),
                            );
                            animations.insert(index, state);
                            Poll::Ready(Some(VecDiff::InsertAt { index, value }))
                        }

                        VecDiff::Push { value } => {
                            let state = Self::animated_state(*duration);
                            let value = callback(
                                value,
                                AnimatedMapBroadcaster(state.animation.raw_clone()),
                            );
                            animations.push(state);
                            Poll::Ready(Some(VecDiff::Push { value }))
                        }

                        VecDiff::UpdateAt { index, value } => {
                            let index = Self::find_index(&animations, index).unwrap_throw();
                            let state = {
                                let state = &animations[index];
                                AnimatedMapBroadcaster(state.animation.raw_clone())
                            };
                            let value = callback(value, state);
                            Poll::Ready(Some(VecDiff::UpdateAt { index, value }))
                        }

                        // TODO test this
                        // TODO should this be treated as a removal + insertion ?
                        VecDiff::Move {
                            old_index,
                            new_index,
                        } => {
                            let old_index = Self::find_index(&animations, old_index).unwrap_throw();

                            let state = animations.remove(old_index);

                            let new_index = Self::find_index(&animations, new_index)
                                .unwrap_or_else(|| animations.len());

                            animations.insert(new_index, state);

                            Poll::Ready(Some(VecDiff::Move {
                                old_index,
                                new_index,
                            }))
                        }

                        VecDiff::RemoveAt { index } => {
                            let index = Self::find_index(&animations, index).unwrap_throw();

                            if Self::should_remove(&mut animations, cx, index) {
                                Self::remove_index(&mut animations, index)
                            } else {
                                continue;
                            }
                        }

                        VecDiff::Pop {} => {
                            let index = Self::find_last_index(&animations).unwrap_throw();

                            if Self::should_remove(&mut animations, cx, index) {
                                Self::remove_index(&mut animations, index)
                            } else {
                                continue;
                            }
                        }

                        // TODO maybe it should play remove animation for this ?
                        VecDiff::Clear {} => {
                            animations.clear();
                            Poll::Ready(Some(VecDiff::Clear {}))
                        }
                    };
                }
                Poll::Ready(None) => {
                    signal.set(None);
                    break;
                }
                Poll::Pending => {
                    is_done = false;
                    break;
                }
            }
        }

        let mut is_removing = false;

        // TODO make this more efficient (e.g. using a similar strategy as FuturesUnordered)
        // This uses rposition so that way it will return VecDiff::Pop in more situations
        let index = animations.iter_mut().rposition(|state| {
            if let Some(ref mut future) = state.removing {
                is_removing = true;
                future.poll_unpin(cx).is_ready()
            } else {
                false
            }
        });

        match index {
            Some(index) => Self::remove_index(&mut animations, index),
            None => {
                if is_done && !is_removing {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Percentage(f64);

impl Percentage {
    pub const START: Percentage = Percentage(0.0);
    pub const END: Percentage = Percentage(1.0);

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
    pub fn map<F>(self, f: F) -> Self
    where
        F: FnOnce(f64) -> f64,
    {
        Self::new(f(self.0))
    }

    #[inline]
    pub fn map_unchecked<F>(self, f: F) -> Self
    where
        F: FnOnce(f64) -> f64,
    {
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
        let mut lock = self.animating.lock().unwrap_throw();
        *lock = None;
    }

    pub fn start(&self) {
        let mut lock = self.animating.lock().unwrap_throw();

        if let None = animating {
            let callback = self.callback.clone();

            let mut starting_time = None;

            *animating = Some(OnTimestampDiff::new(move |value| callback(value)));
        }
    }
}*/

pub fn timestamps_absolute_difference() -> impl Signal<Item = Option<f64>> {
    let mut starting_time = None;

    timestamps().map(move |current_time| {
        current_time.map(|current_time| {
            let starting_time = *starting_time.get_or_insert(current_time);
            current_time - starting_time
        })
    })
}

pub fn timestamps_difference() -> impl Signal<Item = Option<f64>> {
    let mut previous_time = None;

    timestamps().map(move |current_time| {
        let diff = current_time.map(|current_time| {
            previous_time
                .map(|previous_time| current_time - previous_time)
                .unwrap_or(0.0)
        });

        previous_time = current_time;

        diff
    })
}

pub struct OnTimestampDiff(DiscardOnDrop<CancelableFutureHandle>);

impl OnTimestampDiff {
    pub fn new<F>(mut callback: F) -> Self
    where
        F: FnMut(f64) + 'static,
    {
        OnTimestampDiff(spawn_future(timestamps_absolute_difference().for_each(
            move |diff| {
                if let Some(diff) = diff {
                    callback(diff);
                }

                ready(())
            },
        )))
    }
}

impl fmt::Debug for OnTimestampDiff {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_tuple("OnTimestampDiff").finish()
    }
}

#[derive(Debug)]
pub struct MutableAnimationSignal(MutableSignal<Percentage>);

impl Signal for MutableAnimationSignal {
    type Item = Percentage;

    #[inline]
    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        self.0.poll_change_unpin(cx)
    }
}

// TODO verify that this is Sync and Send
struct MutableAnimationState {
    playing: bool,
    duration: f64,
    end: Percentage,
    _animating: Option<OnTimestampDiff>,
}

struct MutableAnimationInner {
    state: Mutex<MutableAnimationState>,
    value: Mutable<Percentage>,
}

// TODO deref to ReadOnlyMutable ?
// TODO provide read_only() method ?
// TODO add `is_playing` method ?
#[derive(Clone)]
pub struct MutableAnimation {
    inner: Arc<MutableAnimationInner>,
}

impl fmt::Debug for MutableAnimation {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let state = self.inner.state.lock().unwrap_throw();

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
                    _animating: None,
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
        lock._animating = None;
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

                    lock._animating =
                        Some(OnTimestampDiff::new(move |diff| {
                            let diff = diff / duration;

                            // TODO test the performance of set_neq
                            if diff >= 1.0 {
                                {
                                    let mut lock = state.inner.state.lock().unwrap_throw();
                                    Self::stop_animating(&mut lock);
                                }
                                state.inner.value.set_neq(Percentage::new_unchecked(end));
                            } else {
                                state.inner.value.set_neq(Percentage::new_unchecked(
                                    range_inclusive(diff, start, end),
                                ));
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

        let mut lock = self.inner.state.lock().unwrap_throw();

        if lock.duration != duration {
            lock.duration = duration;
            self.start_animating(&mut lock);
        }
    }

    #[inline]
    pub fn pause(&self) {
        let mut lock = self.inner.state.lock().unwrap_throw();

        if lock.playing {
            lock.playing = false;
            Self::stop_animating(&mut lock);
        }
    }

    #[inline]
    pub fn play(&self) {
        let mut lock = self.inner.state.lock().unwrap_throw();

        if !lock.playing {
            lock.playing = true;
            self.start_animating(&mut lock);
        }
    }

    fn _jump_to(
        mut lock: &mut MutableAnimationState,
        mutable: &Mutable<Percentage>,
        end: Percentage,
    ) {
        Self::stop_animating(&mut lock);

        lock.end = end;

        mutable.set_neq(end);
    }

    pub fn jump_to(&self, end: Percentage) {
        let mut lock = self.inner.state.lock().unwrap_throw();

        Self::_jump_to(&mut lock, &self.inner.value, end);
    }

    pub fn animate_to(&self, end: Percentage) {
        let mut lock = self.inner.state.lock().unwrap_throw();

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
    pub fn out<F>(p: Percentage, f: F) -> Percentage
    where
        F: FnOnce(Percentage) -> Percentage,
    {
        f(p.invert()).invert()
    }

    pub fn in_out<F>(p: Percentage, f: F) -> Percentage
    where
        F: FnOnce(Percentage) -> Percentage,
    {
        p.map_unchecked(|p| {
            if p <= 0.5 {
                f(Percentage::new_unchecked(p * 2.0)).into_f64() / 2.0
            } else {
                1.0 - (f(Percentage::new_unchecked((1.0 - p) * 2.0)).into_f64() / 2.0)
            }
        })
    }

    /*pub struct Point {
        pub x: f64,
        pub y: f64,
    }*/

    /*impl Point {
        fn range_inclusive(percentage: f64, from: &Self, to: &Self) -> Self {
            Point {
                x: range_inclusive(percentage, from.x, to.x),
                y: range_inclusive(percentage, from.y, to.y),
            }
        }
    }*/

    /*#[inline]
    fn get_values(start: f64, end: f64) -> (f64, f64, f64) {
        let start = 3.0 * start;
        let end = 3.0 * end;
        let a = 1.0 - end + start;
        let b = end - (2.0 * start);
        (a, b, start)
    }

    fn interpolate(p: f64, start: f64, end: f64) -> f64 {
        let (a, b, c) = get_values(start, end);
        ((a * p + b) * p + c) * p
    }

    fn get_slope(p: f64, start: f64, end: f64) -> f64 {
        let (a, b, c) = get_values(start, end);
        (3.0 * a * p * p) + (2.0 * b * p) + c
    }*/

    const EPSILON: f64 = 1e-6;

    pub struct CubicBezier {
        ax: f64,
        bx: f64,
        cx: f64,

        ay: f64,
        by: f64,
        cy: f64,
    }

    impl CubicBezier {
        pub fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
            assert!(x1 >= 0.0 && x1 <= 1.0);
            assert!(y1 >= 0.0 && y1 <= 1.0);
            assert!(x2 >= 0.0 && x2 <= 1.0);
            assert!(y2 >= 0.0 && y2 <= 1.0);

            let cx = 3.0 * x1;
            let bx = 3.0 * (x2 - x1) - cx;
            let ax = 1.0 - cx - bx;

            let cy = 3.0 * y1;
            let by = 3.0 * (y2 - y1) - cy;
            let ay = 1.0 - cy - by;

            Self {
                ax,
                bx,
                cx,
                ay,
                by,
                cy,
            }
        }

        /*fn values(p: f64) -> (f64, f64, f64, f64) {
            let t2 = p * p;
            let one_t = 1.0 - p;
            let one_t2 = one_t * one_t;
            (
                one_t2 * one_t,
                3.0 * one_t2 * p,
                3.0 * one_t * t2,
                t2 * p,
            )
        }*/

        pub fn easing(&self, p: Percentage) -> Percentage {
            // TODO is unchecked okay ?
            p.map_unchecked(|p| {
                if p == 0.0 {
                    0.0
                } else if p == 1.0 {
                    1.0
                } else {
                    self.y(self.get_t_for_x(p))
                }
            })
        }

        /*pub fn point(&self, p: Percentage) -> Point {
            let p = p.into_f64();

            Point {
                x: self.x(p),
                y: self.y(p),
            }
        }*/

        fn x(&self, p: f64) -> f64 {
            ((self.ax * p + self.bx) * p + self.cx) * p

            /*let p1 = range_inclusive(p, self.start.x, self.ctrl1.x);
            let p2 = range_inclusive(p, self.ctrl1.x, self.ctrl2.x);
            let p3 = range_inclusive(p, self.ctrl2.x, self.end.x);

            let p4 = range_inclusive(p, p1, p2);
            let p5 = range_inclusive(p, p2, p3);

            range_inclusive(p, p4, p5)*/
        }

        fn x_derivative(&self, p: f64) -> f64 {
            (3.0 * self.ax * p + 2.0 * self.bx) * p + self.cx
        }

        /*fn x(&self, values: (f64, f64, f64, f64)) -> f64 {
            self.start.x * values.0 +
            self.ctrl1.x * values.1 +
            self.ctrl2.x * values.2 +
            self.end.x * values.3
        }*/

        fn y(&self, p: f64) -> f64 {
            ((self.ay * p + self.by) * p + self.cy) * p

            /*let p1 = range_inclusive(p, self.start.y, self.ctrl1.y);
            let p2 = range_inclusive(p, self.ctrl1.y, self.ctrl2.y);
            let p3 = range_inclusive(p, self.ctrl2.y, self.end.y);

            let p4 = range_inclusive(p, p1, p2);
            let p5 = range_inclusive(p, p2, p3);

            range_inclusive(p, p4, p5)*/
        }

        /*fn y(&self, values: (f64, f64, f64, f64)) -> f64 {
            self.start.y * values.0 +
            self.ctrl1.y * values.1 +
            self.ctrl2.y * values.2 +
            self.end.y * values.3
        }*/

        fn bisect(&self, x: f64) -> f64 {
            let mut start = 0.0;
            let mut end = 1.0;
            let mut t = x;

            debug_assert!(t >= start);
            debug_assert!(t <= end);

            while start < end {
                let x = self.x(t) - x;

                if x.abs() < EPSILON {
                    return t;
                }

                if x > 0.0 {
                    end = t;
                } else {
                    start = t;
                }

                t = (end - start) * 0.5 + start;
            }

            t
        }

        fn get_t_for_x(&self, x: f64) -> f64 {
            let mut t = x;

            // Use Newton's method first, because it's faster
            for _ in 0..8 {
                let x = self.x(t) - x;

                if x.abs() < EPSILON {
                    return t;
                }

                let d = self.x_derivative(t);

                if d.abs() < EPSILON {
                    break;
                }

                t -= x / d;
            }

            // No solution found, bisect instead
            self.bisect(x)
        }

        /*fn get_t_for_x(&self, x: f64) -> f64 {
            const NEWTON_ITERATIONS: usize = 100;

            let mut t = x;

            for _ in 0..NEWTON_ITERATIONS {
                let slope = get_slope(t, self.ctrl1.x, self.ctrl2.x);

                if slope == 0.0 {
                    break;

                } else {
                    let new_x = self.x(t) - x;
                    t -= new_x / slope;
                }
            }

            t
        }*/

        /*fn get_t_for_x(&self, x: f64) -> f64 {
            const MAX_ITERATIONS: usize = 1000;
            //const TOLERANCE: f64 = 0.0001;
            const TOLERANCE: f64 = 0.0000000001;

            let mut start = 0.0;
            let mut end = 1.0;
            let mut iterations = 0;

            loop {
                let t = (end - start) / 2.0 + start;
                let new_x = self.x(t) - x;

                iterations += 1;

                if iterations == MAX_ITERATIONS || new_x.abs() <= TOLERANCE {
                    web_sys::console::log_1(&wasm_bindgen::JsValue::from(&format!("{} {} {} {} {}", iterations, start, end, t, new_x)));
                    return t;

                } else if new_x > 0.0 {
                    end = t;

                } else {
                    start = t;
                }
            }
        }*/
    }
}

/*cubic_bezier(t,
    Percentage::new(1.0),
    Percentage::new(0.0),
    Percentage::new(0.66),
    Percentage::new(0.66),
)*/
