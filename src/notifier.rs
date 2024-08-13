extern crate serde;
extern crate serde_json;

use serde::{Deserialize, Serialize};

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
    nats_connection: async_nats::Client,
    nats_url: String,
    topic: String,
}

impl Notifier {
    pub async fn new(nats_url: String, topic: String) -> Result<Self, async_nats::Error> {
        let nats_connection = async_nats::connect(&nats_url).await?;

        Ok(Self {
            nats_connection,
            nats_url,
            topic
        })
    }
    
    async fn send_message<T>(&self, message: &Message<T>) -> Result<(), async_nats::Error>
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(message)?;

        // Publish the message to the NATS topic
        self.nats_connection
            .publish(self.topic.clone(), serialized.into())
            .await?;
    
        Ok(())
    }

    pub async fn send_hello(&self) -> Result<(), async_nats::Error> {
        let message = Message {
            msg_type: MessageType::<u8>::Hello,
        };

        self.send_message(&message).await
    }

    pub async fn send_update<T>(&self, new_value: T) -> Result<(), async_nats::Error>
    where
        T: Serialize,
    {
        let message = Message {
            msg_type: MessageType::Update { value: new_value },
        };
        self.send_message(&message).await
    }

    pub async fn send_close(&self) -> Result<(), async_nats::Error> {
        let message = Message {
            msg_type: MessageType::<u8>::Close,
        };
        self.send_message(&message).await
    }

    pub fn topic(&self) -> String {
        self.topic.clone()
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
    use std::env;

    // Helper function to create a Notifier instance for testing
    async fn create_test_notifier() -> Notifier {
        let nats_url = env::var("NATS_URL")
                    .unwrap_or_else(|_| "nats://localhost:4222".to_string());

        let notifier = Notifier::new(nats_url, "topic".into()).await.unwrap();
        notifier
    }

    #[tokio::test]
    async fn test_send_hello() {
        let notifier = create_test_notifier().await;
        assert!(notifier.send_hello().await.is_ok());
    }

    #[tokio::test]
    async fn test_send_update() {
        let notifier = create_test_notifier().await;
        let update_message = Message {
            msg_type: MessageType::Update { value: "update".to_string() },
        };
        assert!(notifier.send_message(&update_message).await.is_ok());
    }

    #[tokio::test]
    async fn test_send_close() {
        let notifier = create_test_notifier().await;
        assert!(notifier.send_close().await.is_ok());
    }
}
