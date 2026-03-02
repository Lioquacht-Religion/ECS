// spin_lock.rs

use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct Guard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<'a, T> Guard<'a, T> {
    fn new(lock: &'a SpinLock<T>) -> Self {
        Self { lock }
    }
}

impl<'a, T> Drop for Guard<'a, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

impl<'a, T> Deref for Guard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<'a, T> DerefMut for Guard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: data.into(),
        }
    }

    pub fn get_mut(&mut self) -> &mut T{
        self.data.get_mut()
    }

    pub fn lock<'a>(&'a self) -> Guard<'a, T> {
        while self.locked.swap(true, Ordering::Acquire) {
            std::hint::spin_loop();
        }
        return Guard::new(self);
    }
}

unsafe impl<T: Send> Sync for SpinLock<T> {}

#[cfg(test)]
mod test {
    use crate::utils::spin_lock::SpinLock;

    #[test]
    fn test_spin_lock() {
        let lock_str = SpinLock::new(Vec::new());

        let lock_str = &lock_str;
        std::thread::scope(move |s| {
            
                s.spawn(move || {
                    let mut name = lock_str.lock();
                    name.push(1);
                });
                s.spawn(move || {
                    let mut name = lock_str.lock();
                    name.push(2);
                    name.push(2);
                });
        });

        let v = lock_str.lock();
        assert!(v.as_slice() == [1, 2, 2] || v.as_slice() == [2, 2, 1]);
    }
}
