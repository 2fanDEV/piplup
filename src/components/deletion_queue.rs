use std::collections::VecDeque;

pub struct DeletionQueue {
    queue: VecDeque<Box<dyn Fn() + Send + Sync>>, 
}


impl Default for DeletionQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl DeletionQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new()
        }
    }

    pub fn enqueue<T>(&mut self, func : T) 
    where T: Fn() + 'static + Send + Sync
    {
        self.queue.push_back(Box::new(func));
    }

}
 

