use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
    _id: usize,
    _thread: JoinHandle<()>,
}

impl Worker {
    pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || {
            loop {
                let job = receiver.lock().unwrap().recv().unwrap();
                job();
            }            
        });

        Self {
            _id: id,
            _thread: thread,
        }
    }
}

pub struct ThreadPool {
    sender: mpsc::Sender<Job>,
    _workers: Vec<Worker>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "ThreadPool must have one thread at least");

        let (sender, receiver) = mpsc::channel();
        
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::<Worker>::with_capacity(size);
        for i in 0..size {
            workers.push(Worker::new(i, receiver.clone()));
        }
        Self {
            sender,
            _workers: workers,
        }
    }

    pub fn execute<F>(&self, f: F) 
        where F: FnOnce() + Send + 'static 
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}