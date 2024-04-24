//! Test for analogy viewer with self hosted server.
//!
//! server is expected to listen as "http://localhost:6000"

use std::time::Duration;

use tokio::time::sleep;
use tracing::{error, trace};
use tracing_analogy::AnalogyLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::test]
#[ignore = "works only with installed and activated \"Analogy Viewer\""]
async fn test_message() {
    let layer = AnalogyLayer::new("http://localhost:6000".to_string()).await.unwrap();

    tracing_subscriber::registry()
        .with(EnvFilter::new("error,test=TRACE"))
        .with(layer)
        .init();

    error!(target: "test", "This is a test message");
    trace!(
        target: "test",
        field = 4,
        "This is a test message containing fields"
    );

    sleep(Duration::from_millis(200)).await;
}
