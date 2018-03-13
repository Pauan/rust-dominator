// TODO generalize this so it works for any target, not just JS
// TODO maybe keep a queue in Rust, so that way it only needs to call the callback once
struct Raf(Value);

impl Raf {
    #[inline]
    fn new<F: FnMut(f64, f64) -> bool>(callback: F) -> Self {
        Raf(js!(
            var starting_time = null;
            var callback = @{callback};

            function loop(time) {
                // TODO assign this immediately when the Raf is created ?
                if (starting_time === null) {
                    starting_time = time;
                }

                if (callback(starting_time, time)) {
                    value.id = requestAnimationFrame(loop);

                } else {
                    value.id = null;
                    callback.drop();
                }
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
            var self = @{self};

            if (self.id !== null) {
                cancelAnimationFrame(self.id);
                self.callback.drop();
            }
        }
    }
}


pub trait AnimatedSignalVec {
    fn animated_map<A, B, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
        // TODO maybe don't make this signal generic ?
        where A: Signal<f64>,
              F: FnMut(Self::Item, A) -> B;
}

impl<S: SignalVec> AnimatedSignalVec for S {
    #[inline]
    fn animated_map<A, B, F>(self, duration: f64, f: F) -> AnimatedMap<Self, F>
        // TODO maybe don't make this signal generic ?
        where A: Signal<f64>,
              F: FnMut(Self::Item, A) -> B {
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
    removing: Option<WaitFor<MutableSignal<f64>>>,
}

// TODO move this into signals crate and also generalize it to work with any future, not just animations
pub struct AnimatedMap<A, B> {
    duration: f64,
    animations: Vec<AnimatedMapState>,
    signal: Option<A>,
    callback: B,
}

impl<A, B, F, S> AnimatedMap<S, F>
    where S: SignalVec,
          A: Signal<f64>
          F: FnMut(S::Item, A) -> B {

    #[inline]
    fn call(&self, value: S::Item, state: &AnimatedMapState) -> B {
        (self.callback)(value, state.animation.signal())
    }

    fn animated_state(&self) -> AnimatedMapState {
        let state = AnimatedMapState {
            animation: MutableAnimation::new(self.duration),
            removing: None,
        };

        state.animation.animate_to(1.0);

        state
    }

    fn remove_index(&self, index: usize) -> Async<Option<VecChange<B>>> {
        if index == (self.animations.len() - 1) {
            self.animations.pop();
            Async::Ready(Some(VecChange::Pop {}))

        } else {
            self.animations.remove(index);
            Async::Ready(Some(VecChange::RemoveAt { index }))
        }
    }

    fn remove(&self, index: usize) -> Option<Async<Option<VecChange<B>>>> {
        let state = self.animations[index];

        state.animation.animate_to(0.0);

        let future = state.animation.signal().wait_for(0.0);

        if future.poll().unwrap().is_ready() {
            Some(self.remove_index(index))

        } else {
            state.removing = Some(future);
            None
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

impl<A, B, F, S> SignalVec for AnimatedMap<S, F>
    where S: SignalVec,
          A: Signal<f64>
          F: FnMut(S::Item, A) -> B {
    type Item = B;

    // TODO this can probably be implemented more efficiently
    fn poll(&mut self) -> Async<Option<Self::Item>> {
        let is_done = true;

        // TODO is this loop correct ?
        while let Some(signal) = self.signal.take() {
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
                                        animation: MutableAnimation::new_with_initial(self.duration, 1.0),
                                        removing: None,
                                    };

                                    let value = self.call(value, &state);

                                    self.animations.push(state);

                                    value
                                }).collect()
                            }))
                        },

                        VecChange::InsertAt { index, value } => {
                            let index = self.find_index(index).unwrap_or_else(|| self.animations.len());
                            let state = self.animated_state();
                            let value = self.call(value, &state);
                            self.animations.insert(index, state);
                            Async::Ready(Some(VecChange::InsertAt { index, value }))
                        },

                        VecChange::Push { value } => {
                            let state = self.animated_state();
                            let value = self.call(value, &state);
                            self.animations.push(state);
                            Async::Ready(Some(VecChange::Push { value }))
                        },

                        VecChange::UpdateAt { index, value } => {
                            let index = self.find_index(index).expect("Could not find value");
                            let state = self.animations[index];
                            let value = self.call(value, &state);
                            Async::Ready(Some(VecChange::UpdateAt { index, value }))
                        },

                        VecChange::RemoveAt { index } => {
                            let index = self.find_index(index).expect("Could not find value");

                            if let Some(value) = self.remove(index) {
                                value

                            } else {
                                continue;
                            }
                        },

                        VecChange::Pop {} => {
                            let index = self.find_last_index().expect("Cannot pop from empty vec");

                            if let Some(value) = self.remove(index) {
                                value

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

        let is_removing = false;

        // TODO make this more efficient (e.g. using a similar strategy as FuturesUnordered)
        // This uses rposition so that way it will return VecChange::Pop in more situations
        let index = self.animations.rposition(|state| {
            if let Some(future) = state.removing {
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


#[derive(Debug, Clone, Copy)]
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
    pub fn range_inclusive(&self, low: f64, high: f64) -> f64 {
        low + (self.0 * (high - low))
    }
}

impl Into<f64> for Percentage {
    #[inline]
    fn into(input: Self) -> f64 {
        input.0
    }
}


pub mod unsync {
    pub struct MutableAnimation {
        playing: bool,
        duration: f64,
        value: Mutable<Percentage>,
        end: Percentage,
        raf: Option<Raf>,
    }

    impl MutableAnimation {
        #[inline]
        pub fn new_with_initial(duration: f64, initial: Percentage) -> Self {
            debug_assert!(duration >= 0.0);

            MutableAnimation {
                playing: true,
                duration,
                value: Mutable::new(initial),
                end: initial,
                raf: None,
            }
        }

        #[inline]
        pub fn new(duration: f64) -> Self {
            Self::new_with_initial(duration, Percentage::new_unchecked(0.0))
        }

        #[inline]
        fn stop_raf(&mut self) {
            self.raf.take();
        }

        fn start_raf(&mut self) {
            // TODO use Copy constraint to make value.get() faster ?
            let start = self.value.get();

            if start != self.end && self.duration > 0 {
                self.raf = Some(Raf::new(|starting_time, current_time| {
                    console!(log, format!("{:#?} {:#?}", starting_time, current_time));

                    let diff = (current_time - starting_time) / duration;

                    if diff >= 1 {
                        self.value.set(self.end);
                        false

                    } else {
                        // TODO adjust based on the start / end
                        self.value.set(diff);
                        true
                    }
                }));
            }
        }

        pub fn set_duration(&mut self, duration: f64) {
            debug_assert!(duration >= 0.0);

            if self.duration != duration {
                self.duration = duration;

                // TODO this doesn't need to stop/start in some situations
                if self.playing {
                    self.stop_raf();
                    self.start_raf();
                }
            }
        }

        #[inline]
        pub fn pause(&mut self) {
            self.playing = false;
            self.stop_raf();
        }

        #[inline]
        pub fn play(&mut self) {
            self.playing = true;
            self.start_raf();
        }

        pub fn jump_to(&mut self, end: Percentage) {
            // TODO use Copy constraint to make value.get() faster ?
            if self.value.get() != end {
                self.stop_raf();
                self.end = end;
                self.value.set(end);
            }
        }

        pub fn animate_to(&mut self, end: Percentage) {
            if self.end != end {
                if self.duration <= 0 {
                    self.jump_to(end);

                } else if self.playing {
                    // TODO does this need to stop/start the raf ?
                    self.stop_raf();
                    self.end = end;
                    self.start_raf();
                }
            }
        }
    }
}
