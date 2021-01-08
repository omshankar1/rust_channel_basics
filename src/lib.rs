use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex};

struct Inner<T> {
    queue: VecDeque<T>,
    senders: usize,
}

struct Shared<T> {
    inner: Mutex<Inner<T>>,
    available: Condvar,
}

// Sender<T>
pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut q_lock = self.shared.inner.lock().unwrap();
        q_lock.senders += 1;
        // drop the mutex lock
        drop(q_lock);
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut q_lock = self.shared.inner.lock().unwrap();
        q_lock.senders -= 1;
        let was_last = q_lock.senders == 1;
        if was_last {
            self.shared.available.notify_one();
        }
        drop(q_lock);
    }
}

impl<T: std::fmt::Debug> std::fmt::Display for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let q_lock = self.shared.inner.lock().unwrap();
        let num_senders = q_lock.senders;
        write!(f, "Sender: {}, Deque: {:?}", num_senders, q_lock.queue)
    }
}

impl<T> Sender<T> {
    fn send(&self, msg: T) {
        self.shared.inner.lock().unwrap().queue.push_back(msg);
        drop(self.shared.inner.lock());
        self.shared.available.notify_one();
    }
}

// Receiver<T>
pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
}

impl<T: std::fmt::Debug> std::fmt::Display for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let q_lock = self.shared.inner.lock().unwrap();
        let num_senders = q_lock.senders;
        write!(f, "Sender: {}, Deque: {:?}", num_senders, q_lock.queue)
    }
}
impl<T> Receiver<T> {
    fn receive(&mut self) -> Option<T> {
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
            match inner.queue.pop_front() {
                Some(msg) => return Some(msg),
                None if inner.senders == 0 => {
                    println!("#senders {}", inner.senders);
                    return None;
                }
                None => {
                    inner = self.shared.available.wait(inner).unwrap();
                }
            }
        }
        // None
    }
}
pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Shared {
        inner: Mutex::new(Inner {
            queue: VecDeque::new(),
            senders: 1,
        }),
        available: Condvar::new(),
    };

    let shared = Arc::new(shared);
    (
        Sender {
            // shared: Arc::clone(&shared),
            shared: shared.clone(),
        },
        Receiver {
            // shared: Arc::clone(&shared),
            shared: shared.clone(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ping_pong() {
        let (tx, mut rx) = channel::<usize>();
        // println!("tx {}", tx);
        tx.send(43);
        let tx2 = tx.clone();
        // println!("tx2 {}", tx);
        tx2.send(434);
        tx.send(4399);
        drop(tx2);
        // println!("tx2 after drop {}", tx);
        let result1 = rx.receive();
        let result2 = rx.receive();
        // println!("result: {:?} {:?}", result1, result2);
        assert_eq!(result1, Some(43));
        assert_eq!(result2, Some(434));
    }
    #[test]
    fn drop_tx() {
        let (tx, mut rx) = channel::<usize>();
        println!("tx {}", tx);
        let tx2 = tx.clone();
        tx2.send(443);
        println!("num senders1 {}", rx);
        drop(tx);
        println!("num senders2 {}", rx);
        tx2.send(4434);
        println!("num senders2 {}", rx);
        drop(tx2);
        println!("num senders3 {}", rx);
        let result1 = rx.receive();
        assert_eq!(result1, Some(443));
        let result2 = rx.receive();
        assert_eq!(result2, Some(4434));
        let result3 = rx.receive();
        assert_eq!(result3, None);
    }
}
