// scoped_threadpool.rs

use std::{
    marker::PhantomData,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering}, mpmc::{self, Receiver, Sender}, Arc, Mutex
    },
    thread::{JoinHandle, Thread},
};

type ExecFunc = Box<dyn FnOnce() + Send + Sync + 'static>;

struct Task {
    data: Arc<ScopeData>,
    exec_func: ExecFunc,
}

pub struct ScopedThreadPool {
    sender: Option<Sender<Task>>,
    threads: Vec<JoinHandle<()>>,
}

struct ScopeData {
    pool_thread: Mutex<Thread>,
    num_running_threads: AtomicUsize,
    poison: AtomicBool,
}

pub struct Scope<'scope, 'env: 'scope> {
    pool: &'scope mut ScopedThreadPool,
    data: Arc<ScopeData>,
    _scope_marker: PhantomData<&'scope mut &'scope ()>,
    _env_marker: PhantomData<&'env mut &'env ()>,
}

impl ScopeData {
    fn new(pool_thread: Thread) -> Self {
        Self {
            pool_thread: Mutex::new(pool_thread),
            num_running_threads: AtomicUsize::new(0),
            poison: AtomicBool::new(false),
        }
    }
}

impl<'scope, 'env> Scope<'scope, 'env> {
    fn new(pool: &'scope mut ScopedThreadPool, pool_thread: Thread) -> Self {
        Self {
            pool,
            data: ScopeData::new(pool_thread).into(),
            _scope_marker: PhantomData::default(),
            _env_marker: PhantomData::default(),
        }
    }

    pub fn spawn<F: FnOnce() + Send + Sync + 'scope>(&mut self, exec_func: F) {
        self.data
            .num_running_threads
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        //SAFETY: wait for all tasks submitted in the scope to be finished
        let exec_func = unsafe {
            std::mem::transmute::<
                Box<dyn FnOnce() + Send + Sync + 'scope>,
                Box<dyn FnOnce() + Send + Sync + 'static>,
            >(Box::new(exec_func))
        };
        let task = Task {
            data: self.data.clone(),
            exec_func,
        };
        self.pool.execute(task);
    }
}

struct UnwindGuard<'a> {
    num_running_threads: &'a AtomicUsize,
    poison: &'a AtomicBool,
    pool_thread: &'a Mutex<Thread>,
}

impl<'a> UnwindGuard<'a> {
    fn new(num_running_threads: &'a AtomicUsize, poison: &'a AtomicBool, pool_thread: &'a Mutex<Thread>) -> Self {
        Self {
            num_running_threads,
            poison,
            pool_thread,
        }
    }
}

impl<'a> Drop for UnwindGuard<'a> {
    fn drop(&mut self) {
        if self.num_running_threads.load(Ordering::Acquire) > 0 {
            self.num_running_threads.fetch_sub(1, Ordering::Release);
        } else {
            // should not be zero, if there is still a thread active
            // poison the pool, so that all threads finish execution
            self.poison.store(true, Ordering::Release);
        }
        // thread unwinding has occured, because of a panic,
        // the threadpool needs to be stopped from executing
        self.poison.store(true, Ordering::Release);
        self.pool_thread.lock().unwrap().unpark();
    }
}

impl ScopedThreadPool {
    pub fn new(thread_count: usize) -> Self {
        let mut threads = Vec::with_capacity(thread_count);
        let (mp, mc) = mpmc::channel::<Task>();
        for _i in 0..thread_count {
            let mc = mc.clone();
            let thread = std::thread::spawn(move || {
                Self::worker_loop(mc);
            });
            threads.push(thread);
        }
        Self {
            threads,
            sender: Some(mp),
        }
    }

    fn worker_loop(receiver: Receiver<Task>){
        loop {
            let task = receiver.recv();
            match task {
                Ok(Task { data, exec_func }) => {
                    // handles panics without having the thread and thus the entire scope, be stuck
                    let guard = UnwindGuard::new(&data.num_running_threads, &data.poison, &data.pool_thread);
                    exec_func();
                    if data.num_running_threads.load(Ordering::Acquire) > 0 {
                        data.num_running_threads.fetch_sub(1, Ordering::Release);
                    } else {
                        panic!("Should not be zero, if there is still a thread active.")
                    }
                    // notify pool executor thread that a task has finished
                    data.pool_thread.lock().unwrap().unpark();
                    std::mem::forget(guard);
                }
                Err(_) => break,
            }
        }
    }

    pub fn scope<'env, F, T>(&mut self, f: F) -> T
    where
        F: for<'scope> FnOnce(&'scope mut Scope<'scope, 'env>) -> T,
    {
        let pool_thread = std::thread::current();
        let mut scope = Scope::new(self, pool_thread);
        let scope_data = scope.data.clone();
        let result = f(&mut scope);
        //SAFETY: wait for all tasks submitted in scope to be finished
        loop {
            let poisoned = scope_data.poison.load(Ordering::Acquire);
            if poisoned {
                self.shutdown();
                break;
            }
            let cur_num_running_tasks = scope_data
                .num_running_threads
                .load(std::sync::atomic::Ordering::Acquire);
            if cur_num_running_tasks == 0 {
                break;
            }
            // pool thread gets unparked by worker thread once it finishes executing its task
            // or encountered a panic doing so 
            std::thread::park();
        }
        result
    }

    fn execute(&mut self, task: Task) {
        match self.sender.as_mut().unwrap().send(task) {
            Ok(_) => {}
            Err(_) => {}
        }
    }

    fn shutdown(&mut self){
        drop(self.sender.take());
        while let Some(thread) = self.threads.pop() {
            let _res = thread.join();
        }
    }
}

impl Drop for ScopedThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        while let Some(thread) = self.threads.pop() {
            let _res = thread.join();
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::utils::scoped_threadpool::ScopedThreadPool;

    #[derive(Debug)]
    #[allow(unused)]
    struct Person(String, String, usize);

    #[test]
    fn test_scoped_threadpool() {
        let mut pool = ScopedThreadPool::new(4);
        let mut pers1 = Person("bob".into(), "bab".into(), 10);
        let mut pers3 = Person("baba".into(), "bab".into(), 15);
        let pers4 = Person("buba".into(), "bab".into(), 35);
        for i in 0..1 {
            println!("########## -- loop {i} -- ###########");
            //pool.scope(|scope|{
            pool.scope(|scope| {
                let pers4 = &pers4;
                let pers2 = &mut pers1;
                let pers3 = &mut pers3;
                scope.spawn(move || {
                    std::thread::sleep(Duration::from_millis(300));
                    for _i in 0..15 {
                        println!("Hello there1. {:?}", std::time::Instant::now());
                        pers2.2 += 12;
                    }
                    dbg!(pers2);
                    dbg!(pers4);
                });
                scope.spawn(move || {
                    for _i in 0..20 {
                        println!("Hello there2. {:?}", std::time::Instant::now());
                        pers3.2 += 37;
                    }
                    dbg!(pers3);
                    dbg!(pers4);
                });
                scope.spawn(|| {
                    std::thread::sleep(Duration::from_millis(20));
                    for _i in 0..30 {
                        println!("Hello there3. {:?}", std::time::Instant::now());
                    }
                    println!("Hello there3.");
                });
            });
            dbg!(&pers1);
            dbg!(&pers3);
        }
        //assert!(false);
    }
}
