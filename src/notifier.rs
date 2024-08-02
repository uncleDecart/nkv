extern crate serde;
extern crate serde_json;

use nats::Connection;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
enum MessageType<T>
where
    T: Serialize,
{
    Hello,
    Update { value: T },
    Close,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Message<T>
where
    T: Serialize,
{
    msg_type: MessageType<T>,
}

pub struct Notifier {
    nats_connection: Connection,
    nats_url: String,
    clients: HashSet<String>,
}

impl Notifier {
    pub fn new(nats_url: String) -> Result<Self, Box<dyn Error>> {
        let nats_connection = nats::connect(nats_url.clone())?;

        Ok(Self {
            nats_connection,
            nats_url,
            clients: HashSet::new(),
        })
    }
    
    fn send_message<T>(&self, message: &Message<T>) -> Result<(), Box<dyn Error>>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(message)?;
        for client in &self.clients {
            self.nats_connection.publish(client, serialized.as_bytes())?;
        }
        Ok(())
    }

    pub fn send_hello(&self) -> Result<(), Box<dyn Error>> {
        let message = Message {
            msg_type: MessageType::<u8>::Hello,
        };

        self.send_message(&message)
    }

    pub fn send_update<T>(&self, new_value: T) -> Result<(), Box<dyn Error>>
    where
        T: Serialize,
    {
        let message = Message {
            msg_type: MessageType::Update { value: new_value },
        };
        self.send_message(&message)
    }

    pub fn subscribe(&mut self, client: String) {
        self.clients.insert(client);
    }

    pub fn unsubscribe(&mut self, client: &str) {
        self.clients.remove(client);
    }

    pub fn send_close(&self) -> Result<(), Box<dyn Error>> {
        let message = Message {
            msg_type: MessageType::<u8>::Close,
        };
        self.send_message(&message)
    }

    pub fn addr(&self) -> String {
        self.nats_url.clone()
    }
}

impl Drop for Notifier {
    fn drop(&mut self) {
        let _ = self.send_close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a Notifier instance for testing
    fn create_test_notifier() -> Notifier {
        let mut notifier = Notifier::new("nats://localhost:4222".into()).unwrap();
        notifier.subscribe("client1".to_string());
        notifier.subscribe("client2".to_string());

        notifier
    }

    #[test]
    fn test_send_hello() {
        let notifier = create_test_notifier();
        assert!(notifier.send_hello().is_ok());
    }

    #[test]
    fn test_send_update() {
        let notifier = create_test_notifier();
        let update_message = Message {
            msg_type: MessageType::Update { value: "update".to_string() },
        };
        assert!(notifier.send_message(&update_message).is_ok());
    }

    #[test]
    fn test_send_close() {
        let notifier = create_test_notifier();
        assert!(notifier.send_close().is_ok());
    }
}
