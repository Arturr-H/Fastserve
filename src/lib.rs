#![allow(dead_code, deprecated)]

use std::{ sync::mpsc, thread };
use std::sync::mpsc::Receiver;
use std::sync::{ Arc, Mutex };
use std::thread::JoinHandle;
mod api;


pub struct ThreadHandler {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Job>
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadHandler {

    /*- Create a new ThreadHandler -*/
    /*- num_threads must be greater than 0 -*/
    pub fn new(num_threads:usize) -> ThreadHandler {
        assert!(num_threads > 0);

        let (sender, reciever) = mpsc::channel();
        let reciever:Arc<Mutex<Receiver<Job>>> = Arc::new(Mutex::new(reciever));
        let mut workers:Vec<Worker> = Vec::with_capacity(num_threads);

        /*- Give the workers their tasks -*/
        for id in 0..num_threads {
            workers.push(Worker::new(id, Arc::clone(&reciever)));
        }
        ThreadHandler { workers, sender }
    }

    pub fn exec<F>(&self, f:F) where 
        F:FnOnce() + Send + 'static
    {
        let job:Box<F> = Box::new(f);
        self.sender.send(job).unwrap();
    }
}

struct Worker {
    id:usize,
    thread:thread::JoinHandle<()>
}

impl Worker {
    fn new(id:usize, reciever:Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread:JoinHandle<()> = std::thread::spawn(move || loop {

            /*- Get a job -*/
            let job:Box<dyn FnOnce() + Send> = reciever.lock().unwrap().recv().unwrap();

            job();
        });

        /*- Return the id and the thread that the worker is using -*/
        Worker { id, thread }
    }
}