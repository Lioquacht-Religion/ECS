// one_shot_channel.rs

use std::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, sync::atomic::{AtomicBool, Ordering}, thread::{self, Thread}};

pub struct OneShotChannel<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

pub struct Sender<'a, T>{
    channel: &'a OneShotChannel<T>,
    receiver_thread: Thread,
}

unsafe impl<T: Send> Sync for Sender<'_, T>{
}
unsafe impl<T: Send> Send for Sender<'_, T>{
}

pub struct Receiver<'a, T>{
    channel: &'a OneShotChannel<T>,
    _not_send_marker: PhantomData<*const ()>
}

unsafe impl<T: Send> Sync for Receiver<'_, T>{
}

impl<T> OneShotChannel<T>{
    pub const fn new() -> Self{
        Self { data: UnsafeCell::new(MaybeUninit::uninit()), ready: AtomicBool::new(false) }
    }
    pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>){
        *self = Self::new();
        (Sender::new(self), Receiver::new(self))
    }
}

impl<T> Drop for OneShotChannel<T>{
    fn drop(&mut self) {
        if *self.ready.get_mut(){
            unsafe{
                //SAFETY: atomic bool ready was set to true by writing sender if initialized,
                // or send to false if already received from receiver,
                // so if ready is true, data is currently in an initialized state and needs to be dropped
                self.data.get_mut().assume_init_drop();
            }
        }
    }
}

impl<'a, T> Sender<'a, T> {
    fn new(channel: &'a OneShotChannel<T>) -> Self {
        Self { 
            channel: channel,
            receiver_thread: thread::current(),
        }
    }

    pub fn send(self, data: T){
        unsafe{
            //SAFETY: consumes the sender, no further overwriting of data possible
            // only access to shared reference is reading receiver
            (*self.channel.data.get()).write(data); 
            self.channel.ready.store(true, Ordering::Release);
            self.receiver_thread.unpark();
        }
    }
}

impl<'a, T> Receiver<'a, T> {
    fn new(channel: &'a OneShotChannel<T>) -> Self {
        Self { 
            channel: channel,
            _not_send_marker: PhantomData::default()
        }
    }

    pub fn receive(self) -> T {
        //TODO: handle blocking better by parking/unparking thread
        let mut spin_count = 0;
        while !self.channel.ready.swap(false, Ordering::Acquire) {
            if spin_count < 50 {
                std::hint::spin_loop();
                spin_count += 1;
            }
            else {
                std::thread::park();
            }
        }
        unsafe {
            //SAFETY: read occures after atomic bool ready has been set by initializing sender
            // consumes the reader, split method drops old channel,
            // no further reads of the received shared data possible
            (*self.channel.data.get()).assume_init_read()
        }
    }
}

#[cfg(test)]
mod test{
    use crate::utils::one_shot_channel::OneShotChannel;

    #[test]
    fn test_oneshot_channel() {
        let mut ch = OneShotChannel::new();
        std::thread::scope(|scope|{
            let (s, r) = ch.split();
            scope.spawn(move || {
                s.send(("blabla".to_string(), 34));
            });
            assert_eq!(r.receive(), ("blabla".to_string(), 34));
        });

    }
}
