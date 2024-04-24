use std::{
    collections::HashMap,
    fmt::Display,
    ops::Deref,
    sync::{Arc, Mutex, RwLock},
    time::SystemTime,
};

use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use tracing::{Level, Subscriber};
use tracing_subscriber::{field::VisitOutput, fmt::format::JsonVisitor, Layer};

pub mod analogy {
    #[path = "greet.rs"]
    pub mod greet;
}

use analogy::greet::{analogy_client::AnalogyClient, AnalogyGrpcLogMessage};

#[derive(Debug)]
pub enum Error {
    TonicTransportError(tonic::transport::Error),
    TonicStatus(tonic::Status),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::TonicTransportError(e) => write!(f, "Tonic Transport Error: {e}")?,
            Error::TonicStatus(e) => write!(f, "Tonic Status: {e}")?,
        }
        Ok(())
    }
}
impl std::error::Error for Error {}

impl From<tonic::transport::Error> for Error {
    fn from(value: tonic::transport::Error) -> Self {
        Error::TonicTransportError(value)
    }
}

impl From<tonic::Status> for Error {
    fn from(value: tonic::Status) -> Self {
        Error::TonicStatus(value)
    }
}

struct Event<'a>(&'a tracing::Event<'a>);

pub struct AnalogyLayer {
    tx: Arc<Mutex<ServerState<Sender<AnalogyGrpcLogMessage>>>>,
    destination: String,
}

#[derive(Debug, Clone)]
enum ServerState<T> {
    None,
    Pending,
    Some(T),
}

impl<T> From<Option<T>> for ServerState<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(x) => Self::Some(x),
            None => Self::None,
        }
    }
}

impl AnalogyLayer {
    pub async fn new(dest: String) -> Result<Self, Error> {
        let tx: Option<Sender<AnalogyGrpcLogMessage>> = Self::connect_analogy(dest.clone()).await;

        Ok(Self {
            tx: Arc::new(Mutex::new(tx.into())),
            destination: dest.to_string(),
        })
    }

    fn send_to_server(&self, event: &Event<'_>) {
        let msg: AnalogyGrpcLogMessage = event.into();

        let mut tx = self
            .tx
            .lock()
            .expect("Couldn't get write access to gRpc channel");

        match *tx {
            ServerState::Pending => (),
            ServerState::Some(ref writer) => {
                if writer.capacity() > 0 {
                    writer.try_send(msg).ok();
                }
            }
            ServerState::None => {
                *tx = ServerState::Pending;
                tokio::spawn(Self::restore_connection(self.destination.clone(), self.tx.clone()));
            }
        }
    }

    async fn restore_connection (dest: String, tx_dest: Arc<Mutex<ServerState<Sender<AnalogyGrpcLogMessage>>>>) {
        if let Some(tx) = Self::connect_analogy(dest).await {
            (*tx_dest.lock().expect("Couldn't get write access to gRpc channel")) = ServerState::Some(tx);
        } else {
            (*tx_dest.lock().expect("Couldn't get write access to gRpc channel")) = ServerState::None;    
        }
    }
    
    async fn connect_analogy<'a> (dest: String) -> Option<Sender<AnalogyGrpcLogMessage>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let rx_stream = ReceiverStream::new(rx);

        let endpoint = tonic::transport::Endpoint::from_shared(dest).expect("Failed to create endpoint");
        match AnalogyClient::connect(endpoint).await {
            Ok(mut client) => {
                tokio::spawn(async move {
                    client
                        .subscribe_for_publishing_messages(rx_stream)
                        .await
                        .map_err(|e| {println!("Connection to analogy server lost: {e}")}).ok();
                });
                return Some(tx);
            }
            Err(e) => println!("Failed to connect to analogy server: {e}"),
        };

        None
    }
}

impl<'a> From<&Event<'a>> for AnalogyGrpcLogMessage {
    fn from(value: &Event<'a>) -> Self {
        let level = match *value.0.metadata().level() {
            Level::TRACE => 1,
            Level::DEBUG => 3,
            Level::INFO => 4,
            Level::WARN => 5,
            Level::ERROR => 6,
        };

        let mut fields_str = String::new();
        let mut visitor = JsonVisitor::new(&mut fields_str);
        value.0.record(&mut visitor);
        visitor.finish().ok();

        AnalogyGrpcLogMessage {
            text: fields_str,
            level,
            date: Some(SystemTime::now().into()),
            process_id: std::process::id() as i32,
            thread_id: thread_id::get() as i32,
            module: value
                .0
                .metadata()
                .module_path()
                .unwrap_or_default()
                .to_string(),
            source: value.0.metadata().target().to_string(),
            method_name: "".to_string(),
            file_name: value.0.metadata().file().unwrap_or("").to_string(),
            line_number: value.0.metadata().line().map(|l| l as i32).unwrap_or(-1i32),
            machine_name: gethostname::gethostname().into_string().unwrap_or_default(),
            category: "RUST".to_string(),
            user: "".to_string(),
            additional_information: HashMap::new(),
            id: "".to_string(),
            class: 0,
        }
    }
}

impl<S> Layer<S> for AnalogyLayer
where
    S: Subscriber,
    Self: 'static,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.send_to_server(&Event(event)); // what to do if it fails? we can't log ...
    }
}
