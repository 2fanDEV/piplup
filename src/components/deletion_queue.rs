use std::collections::VecDeque;

pub struct DeletionQueue {
    queue: VecDeque<Box<dyn FnOnce()>>, 
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
   
    pub fn enqueue<T>(&mut self, func: T) where T: FnOnce() + 'static {
        self.queue.push_back(Box::new(func));
    }

    pub fn flush(&mut self) {
        for i in self.queue.drain(..) {
            i()
        }
        self.queue.clear();
    }
}
 

