use std::{
    collections::HashMap,
    fmt::Display,
    time::SystemTime,
};


use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;

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

pub struct AnalogyLayer {
    //    _client: AnalogyClient<Channel>,
    tx: UnboundedSender<AnalogyGrpcLogMessage>,
}

impl AnalogyLayer {
    pub async fn new(dest: &'static str) -> Result<Self, Error> {
        let mut client = AnalogyClient::connect(dest).await?;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let rx_stream = UnboundedReceiverStream::new(rx);

        tokio::spawn(async move {
            client
                .subscribe_for_publishing_messages(rx_stream)
                .await
                .unwrap();
        });

        Ok(Self {
            //            _client: client,
            tx,
        })
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

        let level = match *event.metadata().level() {
            Level::TRACE => 1,
            Level::DEBUG => 3,
            Level::INFO => 4,
            Level::WARN => 5,
            Level::ERROR => 6,
        };

        let mut fields_str = String::new();
        let mut visitor = JsonVisitor::new(&mut fields_str);
        event.record(&mut visitor);
        visitor.finish().ok();

        self.tx
            .send(AnalogyGrpcLogMessage {
                text: fields_str,
                level,
                date: Some(SystemTime::now().into()),
                process_id: std::process::id() as i32,
                thread_id: thread_id::get() as i32,
                module: event.metadata().module_path().unwrap_or_default().to_string(),
                source: event.metadata().target().to_string(),
                method_name: "".to_string(),
                file_name: event.metadata().file().unwrap_or("").to_string(),
                line_number: event.metadata().line().map(|l| l as i32).unwrap_or(-1i32),
                machine_name: gethostname::gethostname().into_string().unwrap_or_default(),
                category: "RUST".to_string(),
                user: "".to_string(),
                additional_information: HashMap::new(),
                id: "".to_string(),
                class: 0,
            })
            .ok(); // what to do if it fails? we can't log ...
    }
}
