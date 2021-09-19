/*
    Copyright 2021, Andrew C. Young <andrew@vaelen.org>

    This file is part of the AR2300 library.

    The AR2300 library is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    Foobar is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with the AR2300 library.  If not, see <https://www.gnu.org/licenses/>.
 */
 
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, Condvar};
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Clone)]
pub struct Queue<T> {
    closed: Arc<AtomicBool>,
    q: Arc<(Mutex<VecDeque<T>>, Condvar)>,
}

impl<T> Queue<T> {
    pub fn new(capacity: usize) -> Self {
        Queue {
            closed: Arc::new(AtomicBool::new(false)),
            q: Arc::new(
                (Mutex::new(
                    VecDeque::with_capacity(capacity)),
                Condvar::new())),
        }
    }
    
    pub fn enqueue(&self, v: T) {
        let (l, cv) = &*self.q;
        let mut queue = l.lock().unwrap();
        let queue_was_empty = queue.is_empty();
        queue.push_back(v);
        if queue_was_empty {
            cv.notify_all();
        }
    }
    
    pub fn dequeue(&self, timeout: Duration) -> Option<T> {
        let (l, cv) = &*self.q;
        let mut queue = cv.wait_timeout_while(
            l.lock().unwrap(), 
            timeout,
            |queue| !self.is_closed() && queue.is_empty()
        ).unwrap().0;
        queue.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        let (l, _) = &*self.q;
        let queue = l.lock().unwrap();
        queue.is_empty()
    }

    pub fn notify_all(&self) {
        let (_, cv) = &*self.q;
        cv.notify_all();
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Relaxed)
    }

    pub fn close(&mut self) {
        self.closed.swap(true, Ordering::Relaxed);
        println!("Queue closed");
    }

}