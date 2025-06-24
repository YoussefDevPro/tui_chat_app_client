use serde::{Deserialize, Serialize};
use std::io::Write;
use std::net::TcpStream;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub sender: u64,
    pub server: u64,
    pub channel: u64,
    pub content: String,
    pub is_replying: bool,
    pub replied_to_msg_id: u64,
}
// welll, this is a data prototype of a message, im gonna make a tui until the server side is done,
// then ill make some function to make things much easier, after ill make the tui, and here things
// will start to be a lil challenging
fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:6969").expect("Could not connect to server");

    let username = 123456789;
    let msg = Message {
        sender: username.clone(),
        server: 1,
        channel: 1,
        content: "Hello!".to_string(),
        is_replying: false,
        replied_to_msg_id: 0,
    };

    let json = serde_json::to_vec(&msg).expect("Failed to serialize");
    stream.write_all(&json).expect("Failed to send message");
} // simple connection with the server in a nice-nice localhost, make a test message then send it
  // to the nice-nice server, just to see if things work
