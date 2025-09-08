
use std::{collections::VecDeque, ops::ControlFlow};
use crate::time::Instant;


#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum TimerId {
  #[cfg(feature = "app_timer")] User(u128),
  #[cfg(all(feature = "frame_pacing", not(target_family = "wasm")))] FrameRequest,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) struct Timeout {
  pub instant: Instant,
  pub id: TimerId,
}


#[derive(Debug, Clone, Default)]
pub(super) struct AppTimer {
  queue: VecDeque<Timeout>,
  set_instant: Option<Option<Instant>>,
}

impl AppTimer {

  pub fn earliest_instant(&self) -> Option<Instant> {
    self.queue.front().map(|t| t.instant)
  }

  pub fn take_set_instant(&mut self) -> Option<Option<Instant>> {
    self.set_instant.take()
  }

  pub fn get_timeout_pos(&mut self, id: TimerId) -> Option<(usize, Timeout)> {
    self.queue.iter().enumerate().find_map(|(i, t)| (id == t.id).then_some((i, *t)))
  }

  pub fn cancel_timeout(&mut self, id: TimerId, if_later: Option<Instant>) -> ControlFlow<Option<Instant>, Instant> {
    if let Some((i, timeout)) = self.get_timeout_pos(id) {

      if let Some(then_instant) = if_later {
        if timeout.instant > then_instant {
          // all is ok, continue
        } else {
          return ControlFlow::Break(Some(timeout.instant));
        }
      }

      self.queue.remove(i);
      if i == 0 { self.set_instant = Some(self.earliest_instant()); }

      ControlFlow::Continue(timeout.instant)
    }
    else { ControlFlow::Break(None) }
  }

  pub fn set_timeout(&mut self, timeout: Timeout, if_earlier: bool) -> ControlFlow<Instant, Option<Instant>> {

    let cancel_if_later = if_earlier.then_some(timeout.instant);

    let canceled = match self.cancel_timeout(timeout.id, cancel_if_later) {
      ControlFlow::Break(None) => ControlFlow::Continue(None),
      ControlFlow::Continue(instant) => ControlFlow::Continue(Some(instant)),
      ControlFlow::Break(Some(instant)) => {
        return ControlFlow::Break(instant);
      },
    };

    if self.queue.is_empty() {
      self.queue.push_back(timeout);
      self.set_instant = Some(Some(timeout.instant));
    }
    else if let Some(i) = self.queue.iter().position(|t| timeout.instant < t.instant) {
      self.queue.insert(i, timeout);
      if i == 0 { self.set_instant = Some(Some(timeout.instant)); }
    } else {
      self.queue.push_back(timeout);
    }

    canceled
  }

  pub fn pop_timeout(&mut self) -> Option<Timeout> {

    if let Some(timeout) = self.queue.front() {
      if timeout.instant <= Instant::now() {
        let popped = self.queue.pop_front();
        self.set_instant = Some(self.earliest_instant());
        return popped;
      }
    }

    None
  }

  pub fn shrink_queue(&mut self) {
    if self.queue.len() < self.queue.capacity() / 2 {
      self.queue.shrink_to((self.queue.capacity() / 4 * 3).max(4));
    }
  }

}