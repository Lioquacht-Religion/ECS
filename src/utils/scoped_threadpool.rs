// scoped_threadpool.rs

use std::{
    marker::PhantomData, sync::{atomic::{AtomicUsize, Ordering}, mpsc::Sender, Arc, Mutex}, thread::JoinHandle
};
type ExecFunc = Box<dyn FnOnce() + Send + Sync + 'static>;

struct Task{
    data: Arc<ScopeData>,
    exec_func: ExecFunc,
}

pub struct ScopedThreadPool {
    sender: Option<Sender<Task>>,
    threads: Vec<JoinHandle<()>>,
}

struct ScopeData{
    num_running_threads: AtomicUsize,
}

pub struct Scope<'scope, 'env: 'scope>{
    pool: &'scope mut ScopedThreadPool,
    data: Arc<ScopeData>,
    _scope_marker : PhantomData<&'scope mut &'scope ()>,
    _env_marker : PhantomData<&'env mut &'env ()>,
}

impl ScopeData{
    fn new() -> Self{
        Self { num_running_threads: AtomicUsize::new(0) }
    }
}

impl<'scope, 'env> Scope<'scope, 'env>{
    fn new(pool: &'scope mut ScopedThreadPool) -> Self{
        Self { 
            pool,
            data: ScopeData::new().into(),
            _scope_marker: PhantomData::default(), 
            _env_marker: PhantomData::default() 
        }
    }

    pub fn spawn<F: FnOnce() + Send + Sync + 'scope>(&mut self, exec_func: F){
        self.data.num_running_threads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        //SAFETY: TODO wait for all tasks submitted in scope to be 
        let exec_func = unsafe { 
            std::mem::transmute::<
                Box<dyn FnOnce() + Send + Sync + 'scope>, 
                Box<dyn FnOnce() + Send + Sync + 'static>
            >(Box::new(exec_func)) 
        };
        let task = Task {
            data: self.data.clone(),
            exec_func
        };
        self.pool.execute(task);
    }
}

impl ScopedThreadPool {
    pub fn new(thread_count: usize) -> Self {
        let mut threads = Vec::with_capacity(thread_count);
        let (mp, sc) = std::sync::mpsc::channel::<Task>();
        let sc = Arc::new(Mutex::new(sc));
        for _i in 0..thread_count {
            let sc = Arc::clone(&sc);
            let thread = std::thread::spawn(move || {
                loop {
                    let task = sc.lock().unwrap().recv();
                    match task {
                        Ok(Task{
                            data, 
                            exec_func
                        }) => {
                            exec_func();
                            if data.num_running_threads.load(Ordering::Relaxed) > 0 {
                                data.num_running_threads.fetch_sub(1, Ordering::Relaxed);
                            }
                            else{
                                panic!("Should not be zero, if there is still a thread active.")
                            }
                        }
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

    pub fn scope<'env, F, T>(&mut self, f: F) -> T
        where 
            F : for<'scope> FnOnce(&'scope mut  Scope<'scope, 'env>) -> T
    {
        //SAFETY: TODO wait for all tasks submitted in scope to be 
        let mut scope = Scope::new(self);
        let scope_data = scope.data.clone();
        let result = f(&mut scope);
        loop {
            let cur_num_running_tasks = scope_data.num_running_threads
                .load(std::sync::atomic::Ordering::Relaxed);
            if cur_num_running_tasks == 0 {
                break;
            }
            //std::thread::yield_now();
        }
        result
    }

    fn execute(&mut self, task: Task) {
        match self.sender.as_mut().unwrap().send(task) {
            Ok(_) => {}
            Err(_) => {}
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
mod test{
    use std::time::Duration;

    use crate::utils::scoped_threadpool::ScopedThreadPool;

    #[derive(Debug)]
    #[allow(unused)]
    struct Person(String, String, usize);

    #[test]
    fn test_scoped_threadpool(){
        let mut pool = ScopedThreadPool::new(4);
        let mut pers1 = Person("bob".into(), "bab".into(), 10);
        let mut pers3 = Person("baba".into(), "bab".into(), 15);
        let pers4 = Person("buba".into(), "bab".into(), 35);
        for i in 0..1 {
            println!("########## -- loop {i} -- ###########");
            //pool.scope(|scope|{
            pool.scope(|scope|{
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
