
// detect changes
#[derive(Debug, Default, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DetectChanges<T: PartialEq + Clone> {
    state: T,
}

impl<T: PartialEq + Clone> DetectChanges<T> {

    pub fn new(initial_state: T) -> Self {
        Self { state: initial_state }
    }

    pub fn state(&self) -> &T {
        &self.state
    }

    pub fn set_state(&mut self, state: T) {
        self.state = state
    }

    pub fn changed(&self, state: &T) -> bool {
        self.state != *state
    }

    pub fn note_change(&mut self, state: &T) -> bool {
        if self.changed(state) {
            self.set_state(state.clone());
            true
        }
        else { false }
    }
}


// once extension
pub use std::sync::Once;

pub trait ButOnce {
    fn call_but_once(&self, func: impl FnOnce());
}

impl ButOnce for Once {
    fn call_but_once(&self, func: impl FnOnce()) {
        if self.is_completed() { func(); }
        else { self.call_once(|| {}); }
    }
}