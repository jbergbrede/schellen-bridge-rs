use warp::{http, Filter};

use futures::{stream::{StreamExt, SplitSink}, SinkExt};
use std::{env, io, str};
use tokio_util::codec::{Decoder, Encoder, Framed};
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use bytes::{BufMut, BytesMut};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyACM0";

struct LineCodec;

impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let newline = src.as_ref().iter().position(|b| *b == b'\n');
        if let Some(n) = newline {
            let line = src.split_to(n + 1);
            return match str::from_utf8(line.as_ref()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Invalid String")),
            };
        }
        Ok(None)
    }
}

impl Encoder<String> for LineCodec {
    type Error = io::Error;

    fn encode(&mut self, item: String, dst: &mut BytesMut) -> Result<(), Self::Error> {
        println!("In writer {:?}", &item);
        dst.reserve(item.len() + 1);
        dst.put(item.as_bytes());
        dst.put_u8(b'\n');
        Ok(())
    }
}

#[derive(Clone)]
struct Sender {
    tx: Arc<Mutex<SplitSink<Framed<SerialStream, LineCodec>, String>>>
}
impl Sender {
    fn new(tx: SplitSink<Framed<SerialStream, LineCodec>, String>) -> Self {
        Sender {
            tx: Arc::new(Mutex::new(tx)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Command {
    Stop,
    Up,
    Down,
}

#[derive(Debug, Deserialize, Serialize)]
struct Payload {
    cmd: Command,
}
impl Payload {
    fn to_payload(&self) -> String {
        match self.cmd {
            Command::Stop => String::from("ss119000000"),
            Command::Up => String::from("ss119010000"),
            Command::Down => String::from("ss119020000"),
        }
    }
}

async fn send_command(
    payload: Payload,
    sender: Sender
) -> Result<impl warp::Reply, warp::Rejection> {

    let write_result = sender.tx.lock().await
            .send(payload.to_payload())
            .await;
    
    match write_result {
        Ok(_) => (),
        Err(err) => println!("{:?}", err),
    }

    Ok(warp::reply::with_status(
        format!("Command: {:?}", payload.cmd),
        http::StatusCode::OK,
    ))
}

#[tokio::main]
async fn main() {
    let mut args = env::args();
    let tty_path = args.nth(1).unwrap_or_else(|| DEFAULT_TTY.into());

    let mut port = tokio_serial::new(tty_path, 9600).open_native_async().unwrap();

    #[cfg(unix)]
    port.set_exclusive(false)
        .expect("Unable to set serial port exclusive to false");

    let stream = LineCodec.framed(port);
    let (tx, mut rx) = stream.split();
    let sender = Sender::new(tx);

    tokio::spawn(async move {
        loop {
            let item = rx
                .next()
                .await
                .expect("Error awaiting future in RX stream.")
                .expect("Reading stream resulted in an error");
            print!("{item}");
        }
    });

    // tokio::spawn(async move {
    //     loop {
    //         let write_result = tx
    //             // .send(String::from(format!("{}\r", r#"print("hello")"#)))
    //             .send(String::from("ss111000000"))
    //             .await;
    //         sleep(Duration::from_secs(2)).await;
    //         match write_result {
    //             Ok(_) => (),
    //             Err(err) => println!("{:?}", err),
    //         }
    //     }
    // });
    
    fn json_body() -> impl Filter<Extract = (Payload,), Error = warp::Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    let with_sender = warp::any().map(move || sender.clone());

    let remote = warp::post()
        .and(warp::path("remote"))
        .and(warp::path::end())
        .and(json_body())
        .and(with_sender.clone())
        .and_then(send_command);

    warp::serve(remote)
        .run(([127, 0, 0, 1], 3030))
        .await;
    
    
}
