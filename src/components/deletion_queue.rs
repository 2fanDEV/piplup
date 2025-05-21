use std::collections::VecDeque;

pub struct DeletionQueue<'a> {
    queue: VecDeque<Box<dyn FnOnce() + 'a>>,
}

impl<'a> Default for DeletionQueue<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DeletionQueue<'a> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn enqueue<T>(mut self, func: T)
    where
        T: FnOnce() + 'a,
    {
        self.queue.push_back(Box::new(func));
    }

    pub fn flush(&mut self) {
        for i in self.queue.drain(..) {
            i()
        }
        self.queue.clear();
    }
}
