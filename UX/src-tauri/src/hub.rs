//! Phát một sự kiện tới mọi đăng ký (nhiều cửa sổ nhận cùng chuỗi sự kiện).
#[allow(unused_imports)]
use crate::dto::events::{ActivationEvent, BootEvent, EmbedEvent, ModelEvent, ScanEvent};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Sink trả `false` khi người nhận đã đóng (cửa sổ tắt): hub bỏ sink đó.
type Sink<E> = Arc<dyn Fn(E) -> bool + Send + Sync>;
/// (id, khoá tuỳ chọn = nhãn webview, sink)
type Entry<E> = (u64, Option<String>, Sink<E>);

pub struct ProgressHub<E: Clone + Send + 'static> {
    next: AtomicU64,
    sinks: Mutex<Vec<Entry<E>>>,
}

impl<E: Clone + Send + 'static> Default for ProgressHub<E> {
    fn default() -> Self {
        ProgressHub { next: AtomicU64::new(1), sinks: Mutex::new(Vec::new()) }
    }
}

impl<E: Clone + Send + 'static> ProgressHub<E> {
    pub fn subscribe(&self, sink: Box<dyn Fn(E) + Send + Sync>) -> u64 {
        self.subscribe_while(Box::new(move |e| {
            sink(e);
            true
        }))
    }

    pub fn subscribe_while(&self, sink: Box<dyn Fn(E) -> bool + Send + Sync>) -> u64 {
        self.add(None, sink)
    }

    /// Đăng ký theo KHOÁ (nhãn webview): đăng ký lại cùng khoá THAY sink cũ, nên tải lại webview hay effect chạy lại không làm
    /// sink chồng lên nhau (sink của trang cũ đã chết nhưng `send` chưa chắc lỗi).
    pub fn subscribe_keyed(&self, key: &str, sink: Box<dyn Fn(E) -> bool + Send + Sync>) -> u64 {
        self.add(Some(key.to_string()), sink)
    }

    fn add(&self, key: Option<String>, sink: Box<dyn Fn(E) -> bool + Send + Sync>) -> u64 {
        let id = self.next.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut g) = self.sinks.lock() {
            if let Some(k) = &key {
                g.retain(|(_, kk, _)| kk.as_deref() != Some(k.as_str()));
            }
            g.push((id, key, Arc::from(sink)));
        }
        id
    }

    pub fn unsubscribe(&self, id: u64) {
        if let Ok(mut g) = self.sinks.lock() {
            g.retain(|(i, _, _)| *i != id);
        }
    }

    pub fn subscribers(&self) -> usize {
        self.sinks.lock().map(|g| g.len()).unwrap_or(0)
    }

    /// Gọi từng sink ngoài khoá (sink chậm hay gọi lại hub không gây deadlock).
    pub fn emit(&self, e: E) {
        let sinks: Vec<(u64, Sink<E>)> = match self.sinks.lock() {
            Ok(g) => g.iter().map(|(i, _, s)| (*i, Arc::clone(s))).collect(),
            Err(_) => return,
        };
        let dead: Vec<u64> = sinks.iter().filter(|(_, s)| !s(e.clone())).map(|(i, _)| *i).collect();
        if !dead.is_empty()
            && let Ok(mut g) = self.sinks.lock()
        {
            g.retain(|(i, _, _)| !dead.contains(i));
        }
    }
}

/// Mọi kênh sự kiện của app (mỗi task thêm kênh của mình).
#[derive(Default)]
pub struct Hubs {
    pub boot: ProgressHub<BootEvent>,
    pub scan: ProgressHub<ScanEvent>,
    pub embed: ProgressHub<EmbedEvent>,
    pub model: ProgressHub<ModelEvent>,
    pub activation: ProgressHub<ActivationEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscribing_again_with_the_same_key_replaces_the_old_sink_but_other_keys_stay() {
        let hub: ProgressHub<u32> = ProgressHub::default();
        let (a, b, c) = (Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())), Arc::new(Mutex::new(Vec::new())));
        let (a2, b2, c2) = (a.clone(), b.clone(), c.clone());
        hub.subscribe_keyed("main", Box::new(move |e| { a2.lock().unwrap().push(e); true }));
        hub.subscribe_keyed("other", Box::new(move |e| { c2.lock().unwrap().push(e); true }));
        hub.emit(1);
        hub.subscribe_keyed("main", Box::new(move |e| { b2.lock().unwrap().push(e); true })); // webview tải lại
        hub.emit(2);
        assert_eq!(hub.subscribers(), 2, "không chồng sink");
        assert_eq!((a.lock().unwrap().clone(), b.lock().unwrap().clone(), c.lock().unwrap().clone()), (vec![1], vec![2], vec![1, 2]));
    }

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
