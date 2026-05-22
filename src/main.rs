mod models;
mod routes;

use std::sync::Arc;
use std::time::Duration;

use axum::routing::{get, post};
use axum::Router;
use nwc::nostr::nips::nip47::NostrWalletConnectUri;
use nwc::NostrWalletConnect;
use tokio::net::TcpListener;
use tokio::sync::broadcast;

use crate::models::EventNotification;
use crate::routes::{map_notification, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rest_nwc_bridge=info,nwc=info".into()),
        )
        .init();

    let nwc_uri = std::env::var("NWC_URI")
        .expect("NWC_URI environment variable is required");

    let uri = NostrWalletConnectUri::parse(&nwc_uri)
        .expect("failed to parse NWC_URI");

    let nwc = NostrWalletConnect::builder(uri)
        .timeout(Duration::from_secs(30))
        .build();

    let (events_tx, _) = broadcast::channel::<EventNotification>(64);

    // Subscribe to wallet notifications
    nwc.subscribe_to_notifications().await?;
    tracing::info!("subscribed to wallet notifications");

    // Spawn background task to forward NWC notifications into the broadcast channel
    let nwc_clone = nwc.clone();
    let tx = events_tx.clone();
    tokio::spawn(async move {
        let result = nwc_clone
            .handle_notifications(|notification| {
                let tx = tx.clone();
                async move {
                    let event =
                        map_notification(notification.notification_type, notification.notification);
                    if let Err(e) = tx.send(event) {
                        tracing::debug!("no active SSE subscribers: {e}");
                    }
                    Ok(false)
                }
            })
            .await;

        if let Err(e) = result {
            tracing::error!("notification handler exited: {e}");
        }
    });

    // Spawn webhook task if WEBHOOK_URL is configured
    if let Ok(webhook_url) = std::env::var("WEBHOOK_URL") {
        let mut webhook_rx = events_tx.subscribe();
        let http_client = reqwest::Client::new();
        tracing::info!("webhook enabled: {webhook_url}");
        tokio::spawn(async move {
            while let Ok(event) = webhook_rx.recv().await {
                if let Err(e) = http_client.post(&webhook_url).json(&event).send().await {
                    tracing::warn!("webhook delivery failed: {e}");
                }
            }
        });
    }

    let state = Arc::new(AppState { nwc, events_tx });

    let app = Router::new()
        .route("/info", get(routes::get_info))
        .route("/balance", get(routes::get_balance))
        .route("/pay", post(routes::pay_invoice))
        .route("/invoice", post(routes::create_invoice))
        .route("/invoice/{payment_hash}", get(routes::lookup_invoice))
        .route("/transactions", get(routes::list_transactions))
        .route("/events", get(routes::sse_events))
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = format!("0.0.0.0:{port}");
    tracing::info!("starting rest-nwc-bridge on {addr}");

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to listen for ctrl+c");
    tracing::info!("shutting down");
}
