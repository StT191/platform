
use std::{rc::Rc, cell::RefCell, collections::VecDeque};
use crate::time::{Instant, Duration};

use super::IdLike;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimeoutResult {
  None,
  Canceled(Instant),
  FoundEarlier(Instant),
}

impl TimeoutResult {
  pub fn was_none(&self) -> bool { matches!(self, TimeoutResult::None) }
  pub fn was_canceld(&self) -> bool { matches!(self, TimeoutResult::Canceled(_)) }
  pub fn found_earlier(&self) -> bool { matches!(self, TimeoutResult::FoundEarlier(_)) }
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(super) struct Timeout<Id: IdLike> {
  pub id: Id,
  pub instant: Instant,
}


pub type RuntimeTimer<T> = Rc<RefCell<Timer<T>>>;


#[derive(Debug, Clone)]
pub struct Timer<Id: IdLike> {
  queue: VecDeque<Timeout<Id>>,
  set_instant: Option<Option<Instant>>,
}

impl<Id: IdLike> Timer<Id> {

  pub(super) fn new() -> Self {
    Self { queue: Default::default(), set_instant: None }
  }

  pub(super) fn earliest_instant(&self) -> Option<Instant> {
    self.queue.front().map(|t| t.instant)
  }

  pub(super) fn take_set_instant(&mut self) -> Option<Option<Instant>> {
    self.set_instant.take()
  }

  pub fn get_timeout(&self, id: &Id) -> Option<Instant> {
    self.queue.iter().find_map(|t| (id == &t.id).then_some(t.instant))
  }

  pub fn cancel_timeout(&mut self, id: &Id, if_later_than: Option<Instant>) -> TimeoutResult {
    if let Some((i, instant)) = self.queue.iter().enumerate()
      .find_map(|(i, t)| (id == &t.id).then_some((i, t.instant)))
    {
      if let Some(threshold) = if_later_than {
        if instant > threshold {
          // all is ok, continue
        } else {
          return TimeoutResult::FoundEarlier(instant);
        }
      }

      self.queue.remove(i);
      if i == 0 { self.set_instant = Some(self.earliest_instant()); }

      TimeoutResult::Canceled(instant)
    }
    else { TimeoutResult::None }
  }

  fn set_timeout_opt_earlier(&mut self, timeout: Timeout<Id>, if_earlier: bool) -> TimeoutResult {

    let cancel_if_later_than = if_earlier.then_some(timeout.instant);

    let timeout_result = self.cancel_timeout(&timeout.id, cancel_if_later_than);

    if timeout_result.found_earlier() {
      return timeout_result;
    }

    if self.queue.is_empty() {
      self.set_instant = Some(Some(timeout.instant));
      self.queue.push_back(timeout);
    }
    else if let Some(i) = self.queue.iter().position(|t| timeout.instant < t.instant) {
      if i == 0 { self.set_instant = Some(Some(timeout.instant)); }
      self.queue.insert(i, timeout);
    } else {
      self.queue.push_back(timeout);
    }

    timeout_result
  }

  pub fn set_timeout(&mut self, id: Id, instant: Instant) -> TimeoutResult {
    self.set_timeout_opt_earlier(Timeout {id, instant}, false)
  }

  pub fn set_timeout_earlier(&mut self, id: Id, instant: Instant) -> TimeoutResult {
    self.set_timeout_opt_earlier(Timeout {id, instant}, true)
  }

  pub fn set_timeout_wait(&mut self, id: Id, duraion: Duration) -> TimeoutResult {
    if let Some(instant) = Instant::now().checked_add(duraion) {
      self.set_timeout_opt_earlier(Timeout {id, instant}, false)
    } else {
      self.cancel_timeout(&id, None)
    }
  }

  pub fn set_timeout_wait_earlier(&mut self, id: Id, duraion: Duration) -> TimeoutResult {
    if let Some(instant) = Instant::now().checked_add(duraion) {
      self.set_timeout_opt_earlier(Timeout {id, instant}, true)
    }
    else if let Some(instant) = self.get_timeout(&id) {
      // MAX is always later than any instant
      TimeoutResult::FoundEarlier(instant)
    }
    else {
      TimeoutResult::None
    }
  }


  fn shrink_queue(&mut self) {
    if self.queue.len() <= self.queue.capacity() / 4 {
      self.queue.shrink_to((self.queue.capacity() / 2).max(1024));
    }
  }

  pub(super) fn pop_timeout(&mut self) -> Option<Timeout<Id>> {

    if let Some(timeout) = self.queue.front() {
      if timeout.instant <= Instant::now() {
        let popped = self.queue.pop_front();
        self.set_instant = Some(self.earliest_instant());
        self.shrink_queue();
        return popped;
      }
    }

    None
  }

}