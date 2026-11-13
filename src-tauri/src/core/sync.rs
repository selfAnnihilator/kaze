//! Poison-tolerant access to `std::sync` locks.
//!
//! A `std::sync` lock is poisoned when a thread panics while holding it. The
//! protected data in this codebase is always plain state (handles, caches,
//! buffers) that stays structurally valid after a panic, so the right reaction
//! is to keep going with the inner guard instead of cascading the panic into
//! every later caller (which would take the playback engine down with it).

use std::sync::{Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

pub trait MutexExt<T: ?Sized> {
    fn lock_or_recover(&self) -> MutexGuard<'_, T>;
}

impl<T: ?Sized> MutexExt<T> for Mutex<T> {
    fn lock_or_recover(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

pub trait RwLockExt<T: ?Sized> {
    fn read_or_recover(&self) -> RwLockReadGuard<'_, T>;
    fn write_or_recover(&self) -> RwLockWriteGuard<'_, T>;
}

impl<T: ?Sized> RwLockExt<T> for RwLock<T> {
    fn read_or_recover(&self) -> RwLockReadGuard<'_, T> {
        self.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write_or_recover(&self) -> RwLockWriteGuard<'_, T> {
        self.write().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn poisoned_locks_still_yield_their_data() {
        let mutex = Arc::new(Mutex::new(7));
        let rwlock = Arc::new(RwLock::new(String::from("kept")));

        let (m, r) = (Arc::clone(&mutex), Arc::clone(&rwlock));
        let joined = std::thread::spawn(move || {
            let _m = m.lock().unwrap();
            let _r = r.write().unwrap();
            panic!("poison both locks");
        })
        .join();
        assert!(joined.is_err());
        assert!(mutex.is_poisoned() && rwlock.is_poisoned());

        *mutex.lock_or_recover() += 1;
        assert_eq!(*mutex.lock_or_recover(), 8);
        rwlock.write_or_recover().push('!');
        assert_eq!(&*rwlock.read_or_recover(), "kept!");
    }
}
