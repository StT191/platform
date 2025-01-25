
use std::{rc::Rc, cell::RefCell, collections::VecDeque, ops::ControlFlow};
use crate::time::{Instant, Duration};

use super::IdLike;

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

  pub fn cancel_timeout(&mut self, id: &Id, if_later: Option<Instant>) -> ControlFlow<Option<Instant>, Instant> {
    if let Some((i, instant)) = self.queue.iter().enumerate()
      .find_map(|(i, t)| (id == &t.id).then_some((i, t.instant)))
    {
      if let Some(then_instant) = if_later {
        if instant > then_instant {
          // all is ok, continue
        } else {
          return ControlFlow::Break(Some(instant));
        }
      }

      self.queue.remove(i);
      if i == 0 { self.set_instant = Some(self.earliest_instant()); }

      ControlFlow::Continue(instant)
    }
    else { ControlFlow::Break(None) }
  }

  fn set_timeout_opt_earlier(&mut self, timeout: Timeout<Id>, if_earlier: bool) -> ControlFlow<Instant, Option<Instant>> {

    let cancel_if_later = if_earlier.then_some(timeout.instant);

    let canceled = match self.cancel_timeout(&timeout.id, cancel_if_later) {
      ControlFlow::Break(None) => ControlFlow::Continue(None),
      ControlFlow::Continue(instant) => ControlFlow::Continue(Some(instant)),
      ControlFlow::Break(Some(instant)) => {
        return ControlFlow::Break(instant);
      },
    };

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

    canceled
  }

  pub fn set_timeout(&mut self, id: Id, instant: Instant) -> Option<Instant> {
    match self.set_timeout_opt_earlier(Timeout {id, instant}, false) {
      ControlFlow::Continue(res) => res,
      ControlFlow::Break(_instant) => unreachable!(),
    }
  }

  pub fn set_timeout_earlier(&mut self, id: Id, instant: Instant) -> ControlFlow<Instant, Option<Instant>> {
    self.set_timeout_opt_earlier(Timeout {id, instant}, true)
  }

  pub fn set_timeout_wait(&mut self, id: Id, duraion: Duration) -> Option<Instant> {
    if let Some(instant) = Instant::now().checked_add(duraion) {
      match self.set_timeout_opt_earlier(Timeout {id, instant}, false) {
        ControlFlow::Continue(res) => res,
        ControlFlow::Break(_) => unreachable!(),
      }
    } else { match self.cancel_timeout(&id, None) {
      ControlFlow::Break(None) => None,
      ControlFlow::Continue(instant) => Some(instant),
      ControlFlow::Break(Some(_)) => unreachable!(),
    }}
  }

  pub fn set_timeout_wait_earlier(&mut self, id: Id, duraion: Duration) -> ControlFlow<Instant, Option<Instant>> {
    if let Some(instant) = Instant::now().checked_add(duraion) {
      self.set_timeout_opt_earlier(Timeout {id, instant}, true)
    }
    else if let Some(instant) = self.get_timeout(&id) {
      // MAX is always later than any instant
      ControlFlow::Break(instant)
    }
    else { match self.cancel_timeout(&id, None) {
      ControlFlow::Break(None) => ControlFlow::Continue(None),
      ControlFlow::Continue(instant) => ControlFlow::Continue(Some(instant)),
      ControlFlow::Break(Some(_)) => unreachable!(),
    }}
  }


  fn shrink_queue(&mut self) {
    if self.queue.len() < self.queue.capacity() / 4 {
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