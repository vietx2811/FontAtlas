//! Phát một sự kiện tới mọi đăng ký (nhiều cửa sổ nhận cùng chuỗi sự kiện).
#[allow(unused_imports)]
use crate::dto::events::{BootEvent, ScanEvent};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

type Sink<E> = Arc<dyn Fn(E) + Send + Sync>;

pub struct ProgressHub<E: Clone + Send + 'static> {
    next: AtomicU64,
    sinks: Mutex<Vec<(u64, Sink<E>)>>,
}

impl<E: Clone + Send + 'static> Default for ProgressHub<E> {
    fn default() -> Self {
        ProgressHub { next: AtomicU64::new(1), sinks: Mutex::new(Vec::new()) }
    }
}

impl<E: Clone + Send + 'static> ProgressHub<E> {
    pub fn subscribe(&self, sink: Box<dyn Fn(E) + Send + Sync>) -> u64 {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut g) = self.sinks.lock() {
            g.push((id, Arc::from(sink)));
        }
        id
    }

    pub fn unsubscribe(&self, id: u64) {
        if let Ok(mut g) = self.sinks.lock() {
            g.retain(|(i, _)| *i != id);
        }
    }

    pub fn subscribers(&self) -> usize {
        self.sinks.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Gọi từng sink ngoài khoá (sink chậm hay gọi lại hub không gây deadlock).
    pub fn emit(&self, e: E) {
        let sinks: Vec<Sink<E>> = match self.sinks.lock() {
            Ok(g) => g.iter().map(|(_, s)| Arc::clone(s)).collect(),
            Err(_) => return,
        };
        for s in sinks {
            s(e.clone());
        }
    }
}

/// Mọi kênh sự kiện của app (mỗi task thêm kênh của mình).
#[derive(Default)]
pub struct Hubs {
    pub boot: ProgressHub<BootEvent>,
    pub scan: ProgressHub<ScanEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_subscriber_gets_same_events_and_unsubscribe_stops() {
        let hub: ProgressHub<u32> = ProgressHub::default();
        let (a, b) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())));
        let (a2, b2) = (a.clone(), b.clone());
        let ia = hub.subscribe(Box::new(move |e| a2.lock().unwrap().push(e)));
        hub.subscribe(Box::new(move |e| b2.lock().unwrap().push(e)));
        hub.emit(1);
        hub.emit(2);
        hub.unsubscribe(ia);
        hub.emit(3);
        assert_eq!(*a.lock().unwrap(), vec![1, 2]);
        assert_eq!(*b.lock().unwrap(), vec![1, 2, 3]);
        assert_eq!(hub.subscribers(), 1);
    }
}
