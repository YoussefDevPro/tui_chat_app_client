use serde::{Deserialize, Serialize};
use std::io::stdin;
use std::io::stdout;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;
use std::net::TcpStream;
use std::thread;

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
//

pub fn input_message() -> Message {
    let mut input = String::new();

    print!("Sender id (u64): ");
    let _ = stdout().flush();
    stdin().read_line(&mut input).unwrap();
    let sender: u64 = input.trim().parse().unwrap();
    input.clear();

    print!("Server id (u64): ");
    let _ = stdout().flush();
    stdin().read_line(&mut input).unwrap();
    let server: u64 = input.trim().parse().unwrap();
    input.clear();

    print!("Channel id (u64): ");
    let _ = stdout().flush();
    stdin().read_line(&mut input).unwrap();
    let channel: u64 = input.trim().parse().unwrap();
    input.clear();

    print!("Content: ");
    let _ = stdout().flush();
    stdin().read_line(&mut input).unwrap();
    let content = input.trim().to_string();
    input.clear();

    print!("Is replying? (true/false): ");
    let _ = stdout().flush();
    stdin().read_line(&mut input).unwrap();
    let is_replying: bool = input.trim().parse().unwrap();
    input.clear();

    print!("Replied to message id (u64): ");
    let _ = stdout().flush();
    stdin().read_line(&mut input).unwrap();
    let replied_to_msg_id: u64 = input.trim().parse().unwrap();

    Message {
        sender,
        server,
        channel,
        content,
        is_replying,
        replied_to_msg_id,
    }
} // this will be removed, this is just for testing

fn main() {
    let stream = TcpStream::connect("127.0.0.1:6969").unwrap();

    let reader_stream = stream.try_clone().unwrap();

    thread::spawn(move || {
        let mut reader = BufReader::new(reader_stream);
        let mut buf = String::new();
        loop {
            buf.clear();
            if reader.read_line(&mut buf).unwrap() == 0 {
                break;
            }
            let reply: Result<Message, _> = serde_json::from_str(&buf);
            if let Ok(msg) = reply {
                println!("Received from server: {:?}", msg);
            } else {
                eprintln!("Received malformed message: {}", buf.trim());
            }
        }
    });

    let mut stream = stream;
    loop {
        let msg = input_message();
        let mut json = serde_json::to_string(&msg).unwrap();
        json.push('\n');
        stream.write_all(json.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
} // simple connection with the server in a nice-nice localhost, make a test message then send it
  // to the nice-nice server, just to see if things work
