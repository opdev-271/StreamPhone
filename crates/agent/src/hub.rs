//! Topic bus: providers publish, WebSocket clients subscribe.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::json;
use tokio::sync::broadcast;

/// `(topic, ready-to-send JSON message)`.
pub type Event = (Arc<str>, Arc<str>);

pub struct Hub {
    tx: broadcast::Sender<Event>,
    last: Mutex<HashMap<String, Arc<str>>>,
    subscribers: Mutex<HashMap<String, usize>>,
}

impl Hub {
    pub fn new() -> Self {
        Self {
            tx: broadcast::channel(256).0,
            last: Mutex::new(HashMap::new()),
            subscribers: Mutex::new(HashMap::new()),
        }
    }

    pub fn listen(&self) -> broadcast::Receiver<Event> {
        self.tx.subscribe()
    }

    /// Publishes a value; identical consecutive values are sent only once.
    pub fn publish<T: Serialize>(&self, topic: &str, payload: &T) {
        let msg: Arc<str> = json!({ "t": "data", "topic": topic, "payload": payload }).to_string().into();
        {
            let mut last = self.last.lock().unwrap();
            if last.get(topic).is_some_and(|prev| **prev == *msg) {
                return;
            }
            last.insert(topic.to_owned(), msg.clone());
        }
        let _ = self.tx.send((topic.into(), msg));
    }

    /// The most recent message of a topic, for clients that subscribe late.
    pub fn last(&self, topic: &str) -> Option<Arc<str>> {
        self.last.lock().unwrap().get(topic).cloned()
    }

    pub fn add_subscriber(&self, topic: &str) {
        *self.subscribers.lock().unwrap().entry(topic.to_owned()).or_default() += 1;
    }

    pub fn remove_subscriber(&self, topic: &str) {
        let mut subs = self.subscribers.lock().unwrap();
        if let Some(n) = subs.get_mut(topic) {
            *n -= 1;
            if *n == 0 {
                subs.remove(topic);
                // Drop the cached value so a later subscriber never sees stale data.
                self.last.lock().unwrap().remove(topic);
            }
        }
    }

    /// Providers skip their work when nobody is looking.
    pub fn has_subscribers(&self, topic: &str) -> bool {
        self.subscribers.lock().unwrap().contains_key(topic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedupes_and_tracks_subscribers() {
        let hub = Hub::new();
        let mut rx = hub.listen();
        hub.add_subscriber("a");
        assert!(hub.has_subscribers("a"));

        hub.publish("a", &1);
        hub.publish("a", &1);
        hub.publish("a", &2);
        assert!(rx.try_recv().unwrap().1.contains("\"payload\":1"));
        assert!(rx.try_recv().unwrap().1.contains("\"payload\":2"));
        assert!(rx.try_recv().is_err());
        assert!(hub.last("a").is_some());

        hub.remove_subscriber("a");
        assert!(!hub.has_subscribers("a"));
        assert!(hub.last("a").is_none());
    }
}
