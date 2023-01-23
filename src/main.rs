mod serial;
use crate::serial::io::{Connection, Sender};
use warp::{http, Filter, Rejection};

use futures::{stream::StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use std::{env, str};

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyACM0";

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

async fn send_command(payload: Payload, sender: Sender) -> Result<impl warp::Reply, Rejection> {
    let write_result = sender.tx.lock().await.send(payload.to_payload()).await;

    match write_result {
        Ok(_) => Ok(warp::reply::with_status(
            format!("Command: {:?}", payload.cmd),
            http::StatusCode::OK,
        )),
        Err(_err) => Err(warp::reject::reject()),
    }
}

#[tokio::main]
async fn main() {
    let mut args = env::args();
    let tty_path = args.nth(1).unwrap_or_else(|| DEFAULT_TTY.into());

    let mut conn = Connection::new(tty_path);

    tokio::spawn(async move {
        loop {
            let item = conn
                .rx
                .next()
                .await
                .expect("Error awaiting future in RX stream.")
                .expect("Reading stream resulted in an error");
            print!("{item}");
        }
    });

    fn json_body() -> impl Filter<Extract = (Payload,), Error = Rejection> + Clone {
        // When accepting a body, we want a JSON body
        // (and to reject huge payloads)...
        warp::body::content_length_limit(1024 * 16).and(warp::body::json())
    }

    let with_sender = warp::any().map(move || conn.sender.clone());

    let remote = warp::post()
        .and(warp::path("remote"))
        .and(warp::path::end())
        .and(json_body())
        .and(with_sender.clone())
        .and_then(send_command);

    warp::serve(remote).run(([127, 0, 0, 1], 3030)).await;
}
