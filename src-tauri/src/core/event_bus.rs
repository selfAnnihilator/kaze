use crate::core::error::AppResult;
use crate::core::event::Event;
use tokio::sync::broadcast;
use tracing::debug;

/// Default capacity for the event broadcast channel.
pub const DEFAULT_EVENT_BUS_CAPACITY: usize = 1024;

/// Asynchronous, decoupled event dispatcher backed by a Tokio broadcast channel.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<Event>,
}

impl EventBus {
    /// Creates a new EventBus with the specified channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publishes a domain event across the event bus to all active subscribers.
    pub fn publish(&self, event: Event) -> AppResult<usize> {
        debug!(?event, "Publishing domain event to EventBus");
        if self.sender.receiver_count() == 0 {
            return Ok(0);
        }
        match self.sender.send(event) {
            Ok(count) => Ok(count),
            Err(_) => Ok(0),
        }
    }

    /// Subscribes to the event stream, returning a broadcast Receiver.
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    /// Returns the number of currently active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(DEFAULT_EVENT_BUS_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_publish_and_subscribe() {
        let bus = EventBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let event = Event::PlaybackStopped { session_id: "sess_test".to_string() };
        let subscriber_count = bus.publish(event.clone()).unwrap();

        assert_eq!(subscriber_count, 2);
        assert_eq!(rx1.recv().await.unwrap(), event);
        assert_eq!(rx2.recv().await.unwrap(), event);
    }
}
