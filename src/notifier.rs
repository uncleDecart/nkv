extern crate zmq;
extern crate serde;
extern crate serde_json;

use serde::{Serialize, Deserialize};
use zmq::Socket;

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
    socket: Socket,
}

impl Notifier {
    pub fn new(ctx: &zmq::Context, address: &str) -> Notifier {
        let socket = ctx.socket(zmq::PUB).unwrap();
        socket.bind(address).unwrap();
        Notifier { socket }
    }
    
    fn send_message<T>(&self, message: &Message<T>)
    where
        T: Serialize,
    {
        let serialized = serde_json::to_string(message).unwrap();
        self.socket.send(&serialized, 0).unwrap();
    }

    pub fn send_hello(&self) {
        let message = Message {
            msg_type: MessageType::<u8>::Hello,
        };

        self.send_message(&message);
    }

    pub fn send_update<T>(&self, new_value: T)
    where
        T: Serialize,
    {
        let message = Message {
            msg_type: MessageType::Update { value: new_value },
        };
        self.send_message(&message);
    }

    pub fn send_close(&self) {
        let message = Message {
            msg_type: MessageType::<u8>::Close,
        };
        self.send_message(&message);
    }
}

impl Drop for Notifier {
    fn drop(&mut self) {
        self.send_close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn setup_subscriber(ctx: &zmq::Context, address: &str) -> Socket {
        let subscriber = ctx.socket(zmq::SUB).unwrap();
        subscriber.connect(address).unwrap();
        subscriber.set_subscribe(b"").unwrap();
        subscriber
    }

    #[test]
    fn test_send_hello() {
        let context = zmq::Context::new();
        let address = "ipc:///tmp/test_send_hello";
        let notifier = Notifier::new(&context, address);
        let subscriber = setup_subscriber(&context, address);

        // Allow some time for the connection to be established
        thread::sleep(Duration::from_millis(100));

        notifier.send_hello();

        let message = subscriber.recv_string(0).unwrap().unwrap();
        let received_message: Message<()> = serde_json::from_str(&message).unwrap();
        assert_eq!(received_message.msg_type, MessageType::Hello);
    }

    #[test]
    fn test_send_update() {
        let context = zmq::Context::new();
        let address = "ipc:///tmp/test_send_update";
        let notifier = Notifier::new(&context, address);
        let subscriber = setup_subscriber(&context, address);

        // Allow some time for the connection to be established
        thread::sleep(Duration::from_millis(100));

        notifier.send_update("test_value".to_string());

        let message = subscriber.recv_string(0).unwrap().unwrap();
        let received_message: Message<String> = serde_json::from_str(&message).unwrap();
        assert_eq!(received_message.msg_type, MessageType::Update { value: "test_value".to_string() });
    }

    #[test]
    fn test_send_close() {
        let context = zmq::Context::new();
        let address = "ipc:///tmp/test_send_close";
        let notifier = Notifier::new(&context, address);
        let subscriber = setup_subscriber(&context, address);

        // Allow some time for the connection to be established
        thread::sleep(Duration::from_millis(100));

        notifier.send_close();

        let message = subscriber.recv_string(0).unwrap().unwrap();
        let received_message: Message<()> = serde_json::from_str(&message).unwrap();
        assert_eq!(received_message.msg_type, MessageType::Close);
    }
}