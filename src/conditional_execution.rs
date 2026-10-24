
// detect changes
pub trait DetectChanges: PartialEq + Clone {

    #[inline] fn changed(&self, state: &Self) -> bool {
        self != state
    }

    #[inline] fn note_change(&mut self, state: &Self) -> bool {
        if self.changed(state) {
            *self = state.clone();
            true
        }
        else { false }
    }
}

impl<T: PartialEq + Clone> DetectChanges for T {}


// once extension
pub use std::sync::Once;

pub trait ButOnce {
    fn call_but_once(&self, func: impl FnOnce());
}

impl ButOnce for Once {
    #[inline] fn call_but_once(&self, func: impl FnOnce()) {
        if self.is_completed() { func(); }
        else { self.call_once(|| {}); }
    }
}


// flip boolean extension
pub trait BoolFlip {
    fn set(&mut self) -> bool;
    fn take(&mut self) -> bool;
    fn flip(&mut self) -> bool;
}

impl BoolFlip for bool {

    #[inline] fn set(&mut self) -> bool {
        if *self == false {
            *self = true;
            true
        }
        else { false }
    }

    #[inline] fn take(&mut self) -> bool {
        if *self == true {
            *self = false;
            true
        }
        else { false }
    }

    #[inline] fn flip(&mut self) -> bool {
        *self = !*self;
        *self
    }
}