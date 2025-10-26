
use std::{task::*, sync::Arc, pin::Pin, marker::Unpin};

use super::*;

struct RuntimeFutureWaker<I: IdLike + Clone, U: EventLike> {
  id: I,
  event_loop_proxy: RuntimeEventLoopProxy<I, U>,
}

impl<I: IdLike + Clone, U: EventLike> Wake for RuntimeFutureWaker<I, U> {
  fn wake_by_ref(self: &Arc<Self>) {
    self.event_loop_proxy
      .send_event(RuntimeEventExt::WakeFuture(self.id.clone()))
      .unwrap_or_else(|m| log::error!("{m:?}"))
    ;
  }
  fn wake(self: Arc<Self>) { Self::wake_by_ref(&self) }
}


pub trait Futures {
  type Id: IdLike + Clone;
  type Future: Future<Output: EventLike> + Unpin + 'static;
  fn new() -> Self;
  fn spawn(&mut self, future: Self::Future) -> Self::Id;
  fn insert(&mut self, id: Self::Id, future: Self::Future);
  fn fetch(&mut self, id: &Self::Id) -> Option<Self::Future>;
  fn clean(&mut self); // clean up if neccessary
}


use std::{rc::Rc, cell::RefCell};

#[derive(Debug)]
pub struct FutureRuntime<F: Futures, U: EventLike> {
  futures: Rc<RefCell<F>>,
  event_loop_proxy: RuntimeEventLoopProxy<F::Id, U>,
}

impl<F: Futures, U: EventLike> Clone for FutureRuntime<F, U> {
  fn clone(&self) -> Self {
    Self {
      futures: Rc::clone(&self.futures),
      event_loop_proxy: self.event_loop_proxy.clone()
    }
  }
}

impl<F: Futures, U: EventLike> FutureRuntime<F, U> {

  pub fn new(event_loop_proxy: RuntimeEventLoopProxy<F::Id, U>) -> Self {
    Self {futures: RefCell::new(F::new()).into(), event_loop_proxy}
  }

  pub fn poll(&self, id: &F::Id) -> Option<<F::Future as Future>::Output> {

    let Some(mut future) = self.futures.borrow_mut().fetch(id) else {
      log::error!("future {id:?} not found");
      return None;
    };

    let waker = Arc::new(RuntimeFutureWaker {
      id: id.clone(), event_loop_proxy: self.event_loop_proxy.clone(),
    }).into();

    let mut context = Context::from_waker(&waker);

    match Pin::new(&mut future).poll(&mut context) {
      Poll::Ready(res) => {
        self.futures.borrow_mut().clean();
        Some(res)
      },
      Poll::Pending => {
        self.futures.borrow_mut().insert(id.clone(), future);
        None
      },
    }
  }

  pub fn spawn(&self, future: F::Future) -> F::Id {
    let id = self.futures.borrow_mut().spawn(future);
    // wake future the first time
    self.event_loop_proxy
      .send_event(RuntimeEventExt::WakeFuture(id.clone()))
      .unwrap_or_else(|m| log::error!("{m:?}"))
    ;
    id
  }

  pub fn cancel(&self, id: &F::Id) {
    self.futures.borrow_mut().fetch(id);
  }

}


// timeout
#[cfg(any(feature="timeout", feature="async_timeout", feature="frame_pacing"))]
mod async_timeout {

  use super::*;
  use crate::time::*;


  #[derive(Debug, Clone)]
  pub struct AsyncTimeoutId(Waker);

  impl AsyncTimeoutId {
    pub fn wake(self) { self.0.wake() }
  }

  impl PartialEq for AsyncTimeoutId {
    fn eq(&self, other: &Self) -> bool { self.0.data() == other.0.data() }
  }
  impl Eq for AsyncTimeoutId {}

  impl Hash for AsyncTimeoutId {
    fn hash<H: std::hash::Hasher>(&self, hasher: &mut H) { self.0.data().hash(hasher) }
  }


  pub struct AsyncTimeout<T: IdLike + From<AsyncTimeoutId>> {
    instant: Instant,
    timer: Option<RuntimeTimer<T>>,
  }

  impl<T: IdLike + From<AsyncTimeoutId>> AsyncTimeout<T> {
    pub fn new(timer: RuntimeTimer<T>, instant: Instant) -> Self {
      Self {timer: Some(timer), instant}
    }
  }

  impl<T: IdLike + From<AsyncTimeoutId>> Future for AsyncTimeout<T> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context) -> Poll<()> {

      if Instant::now() >= self.instant {
        return Poll::Ready(());
      }

      if let Some(timer) = self.timer.take() {
        let id = AsyncTimeoutId(context.waker().clone());
        timer.borrow_mut().set_timeout(T::from(id), self.instant);
      }

      Poll::Pending
    }
  }


  pub struct AsyncTimer<T: IdLike + From<AsyncTimeoutId>> {
    timer: RuntimeTimer<T>,
  }

  impl<T: IdLike + From<AsyncTimeoutId>> AsyncTimer<T> {

    pub fn new(timer: RuntimeTimer<T>) -> Self { Self {timer} }

    pub fn timeout(&self, instant: Instant) -> AsyncTimeout<T> {
      AsyncTimeout::new(self.timer.clone(), instant)
    }

    pub fn wait(&self, duraion: Duration) -> Option<AsyncTimeout<T>> {
      Instant::now().checked_add(duraion).map(|instant| self.timeout(instant))
    }

  }

}

#[cfg(any(feature="timeout", feature="async_timeout", feature="frame_pacing"))]
pub use async_timeout::*;