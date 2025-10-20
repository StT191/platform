
use crate::*;

#[cfg(feature = "async_timeout")]
impl From<AsyncTimeoutId> for AppTimeoutId {
  fn from(id: AsyncTimeoutId) -> Self { Self::Async(id) }
}


// simple engine
#[cfg(not(feature = "futures"))]
mod single {
  use super::*;

  pub struct AppFutures(Option<BoxFuture<'static, ()>>);

  impl RuntimeFutures for AppFutures {

    type Id = ();
    type Future = AppFuture;

    fn new() -> Self { Self(None) }
    fn spawn(&mut self, future: Self::Future) { self.0 = Some(future) }
    fn insert(&mut self, _id: Self::Id, future: Self::Future) { self.0 = Some(future) }
    fn fetch(&mut self, _id: &Self::Id) -> Option<Self::Future> { self.0.take() }
    fn clean(&mut self) {}
  }

}

#[cfg(not(feature = "futures"))]
pub use single::*;


// multi engine
#[cfg(feature = "futures")]
mod mapped {
  use super::*;
  use crate::rapidhash::RapidHashMap;

  #[derive(Default)]
  pub struct AppFutures {
    futures: RapidHashMap<u64, BoxFuture<'static, ()>>,
    next_id: u64,
  }

  #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
  pub struct AppFutureId(u64);

  impl RuntimeFutures for AppFutures {

    type Id = AppFutureId;
    type Future = AppFuture;

    fn new() -> Self { Default::default() }

    fn insert(&mut self, id: Self::Id, future: Self::Future) {
      self.futures.insert(id.0, future);
    }

    fn spawn(&mut self, future: Self::Future) -> Self::Id {
      let id = self.next_id;
      self.futures.insert(id, future);
      self.next_id += 1;
      AppFutureId(id)
    }

    fn fetch(&mut self, id: &Self::Id) -> Option<Self::Future> {
      self.futures.remove(&id.0)
    }

    fn clean(&mut self) {
      // shrink hash-map if advisable
      if self.futures.len() <= self.futures.capacity() / 4 {
        self.futures.shrink_to((self.futures.capacity() / 2).max(1024));
      }
    }
  }
}

#[cfg(feature = "futures")]
pub use mapped::*;
