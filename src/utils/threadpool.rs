// threadpool.rs

use std::{
    sync::{Arc, Mutex, mpsc::Sender},
    thread::JoinHandle,
};

type ExecFunc = Box<dyn FnOnce() + Send + Sync + 'static>;

pub struct ThreadPool {
    threads: Vec<JoinHandle<()>>,
    sender: Option<Sender<ExecFunc>>,
}

impl ThreadPool {
    pub fn new(thread_count: usize) -> Self {
        let mut threads = Vec::with_capacity(thread_count);
        let (mp, sc) = std::sync::mpsc::channel::<ExecFunc>();
        let sc = Arc::new(Mutex::new(sc));
        for _i in 0..thread_count {
            let sc = Arc::clone(&sc);
            let thread = std::thread::spawn(move || {
                loop {
                    let exec_func = sc.lock().unwrap().recv();
                    match exec_func {
                        Ok(exec_func) => exec_func(),
                        Err(_) => break,
                    }
                }
            });
            threads.push(thread);
        }
        Self {
            threads,
            sender: Some(mp),
        }
    }

    pub fn execute(&mut self, exec_func: impl FnOnce() + Send + Sync + 'static) {
        match self.sender.as_mut().unwrap().send(Box::new(exec_func)) {
            Ok(_) => {}
            Err(_) => {}
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        while let Some(thread) = self.threads.pop() {
            let _res = thread.join();
        }
    }
}
