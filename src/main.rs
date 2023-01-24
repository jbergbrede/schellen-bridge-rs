#[macro_use]
extern crate rocket;
mod serial;
use crate::serial::serde::LineCodec;

use color_eyre::eyre::Result;
use futures::{stream::SplitSink, SinkExt};
use rocket::{
    futures::StreamExt,
    tokio::{
        self,
        time::{sleep, Duration},
    },
    State,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{Decoder, Framed};
use tracing::info;

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyACM0";

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/shutter/<action>")]
async fn action(
    action: &str,
    tx: &State<Arc<Mutex<SplitSink<Framed<SerialStream, LineCodec>, String>>>>,
) -> Option<String> {
    let payload = match action.to_lowercase().as_str() {
        "up" => String::from("ss119010000"),
        "down" => String::from("ss119020000"),
        "stop" => String::from("ss119000000"),
        _ => String::from("init"),
    };

    let write_result = tx.lock().await.send(payload).await;

    match write_result {
        Err(e) => Some(format!("Write failed! {:?}", e)),
        Ok(_) => Some("OK\n".to_string()),
    }
}

#[get("/delay/<seconds>")]
async fn delay(seconds: u64) -> String {
    sleep(Duration::from_secs(seconds)).await;
    format!("Waited for {} seconds", seconds)
}

#[rocket::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let mut port = tokio_serial::new(DEFAULT_TTY, 9600)
        .open_native_async()
        .expect(format!("Unable to open serial port: {}", DEFAULT_TTY).as_str());

    #[cfg(unix)]
    port.set_exclusive(false)
        .expect("Unable to set serial port exclusive to false");

    let stream = LineCodec.framed(port);
    let (tx, mut rx) = stream.split();

    tokio::spawn(async move {
        loop {
            let item = rx
                .next()
                .await
                .expect("Error awaiting future in RX stream.")
                .expect("Reading stream resulted in an error");
            info!("{item}");
        }
    });

    let _rocket = rocket::build()
        .manage(Arc::new(Mutex::new(tx)))
        .mount("/", routes![index, action, delay])
        .launch()
        .await?;

    Ok(())
}
