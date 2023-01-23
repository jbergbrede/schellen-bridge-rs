pub mod serde {
    use bytes::{BufMut, BytesMut};
    use std::io;
    use std::str;
    use tokio_util::codec::{Decoder, Encoder};

    pub struct LineCodec;

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
}

pub mod io {
    use crate::serial::serde::LineCodec;
    use futures::stream::{SplitSink, SplitStream};
    use futures::StreamExt;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio_serial::{SerialPortBuilderExt, SerialStream};
    use tokio_util::codec::{Decoder, Framed};

    #[derive(Clone)]
    pub struct Sender {
        pub tx: Arc<Mutex<SplitSink<Framed<SerialStream, LineCodec>, String>>>,
    }
    impl Sender {
        fn new(tx: SplitSink<Framed<SerialStream, LineCodec>, String>) -> Self {
            Sender {
                tx: Arc::new(Mutex::new(tx)),
            }
        }
    }

    pub struct Connection {
        pub sender: Sender,
        pub rx: SplitStream<Framed<SerialStream, LineCodec>>,
    }
    impl Connection {
        pub fn new(tty_path: String) -> Self {
            let mut port = tokio_serial::new(tty_path, 9600)
                .open_native_async()
                .unwrap();

            #[cfg(unix)]
            port.set_exclusive(false)
                .expect("Unable to set serial port exclusive to false");

            let stream = LineCodec.framed(port);
            let (tx, rx) = stream.split();

            Connection {
                sender: Sender::new(tx),
                rx,
            }
        }
    }
}
