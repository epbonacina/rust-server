use std::thread;
use std::sync::{mpsc, Arc, Mutex};


type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker {
   id: usize,
   thread: Option<thread::JoinHandle<()>>
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            match message {
                Ok(job) => {
                    println!("Worker {id} got a job; executing.");
                    job();
                }
                Err(_) => {
                    println!("Worker {id} disconnected; shutting down.");
                    break;
                }
            }
        });
        Worker { id, thread: Some(thread) }
    }
}


pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>
}

impl ThreadPool {
    pub fn new(number_of_threads: usize) -> ThreadPool {
        assert!(number_of_threads > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        // This is an optimization. Vec::new could've been used.
        let mut workers = Vec::with_capacity(number_of_threads); 
        for worker_id in 0..number_of_threads {
            workers.push(Worker::new(worker_id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender: Some(sender) }
    }

    pub fn execute<F>(&self, f: F)
    where
        // We use `FnOnce()` here because it's the same closure type used in `thread::spawn`,
        // we use `Send` here because we need to transfer the closure from one thread to another
        // and we use `'static` here because we don't know how long the thread will take to
        // execute.
        F: FnOnce() + Send + 'static, 
    {
        let job = Box::new(f);

        self.sender
            .as_ref()
            .unwrap()
            .send(job)
            .unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}
