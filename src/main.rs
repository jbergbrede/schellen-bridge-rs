#[macro_use]
extern crate rocket;
use color_eyre::eyre::Result;
use futures::{stream::SplitSink, SinkExt};
use rocket::{futures::StreamExt, tokio, State};
use schellen_bridge_rs::LineCodec;
use std::io::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use tokio_util::codec::{Decoder, Framed};
use tracing::info;

#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyACM0";

#[get("/")]
fn index() -> &'static str {
    "This is an API for the Schellenberg Stick."
}

#[get("/shutter/<cmd>")]
async fn shutter(
    cmd: &str,
    tx: &State<Arc<Mutex<SplitSink<Framed<SerialStream, LineCodec>, String>>>>,
) -> Result<Option<String>, Error> {
    let payload = match cmd.to_lowercase().as_str() {
        "up" => Some(String::from("ss119010000")),
        "down" => Some(String::from("ss119020000")),
        "stop" => Some(String::from("ss119000000")),
        "init" => Some(String::from("init")),
        _ => None,
    };

    match payload {
        Some(str) => {
            tx.lock().await.send(str).await?;
            Ok(Some(String::from("OK\n")))
        }
        None => Ok(None),
    }
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
        .mount("/", routes![index, shutter])
        .launch()
        .await?;

    Ok(())
}
