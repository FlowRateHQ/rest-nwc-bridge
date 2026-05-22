use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive};
use axum::response::{IntoResponse, Response, Sse};
use axum::Json;
use nostr::nips::nip47::{
    ListTransactionsRequest, LookupInvoiceRequest, LookupInvoiceResponse, MakeInvoiceRequest,
    NotificationResult, NotificationType, PayInvoiceRequest, TransactionState, TransactionType,
};
use nwc::NostrWalletConnect;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::models::{
    BalanceResponse, CreateInvoiceRequest, CreateInvoiceResponse, ErrorResponse,
    EventNotification, InfoResponse, InvoiceResponse, PayRequest, PayResponse, TransactionQuery,
    TransactionsResponse,
};

// -- App state --

pub struct AppState {
    pub nwc: NostrWalletConnect,
    pub events_tx: broadcast::Sender<EventNotification>,
}

// -- Error handling --

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("request failed: {:#}", self.0);
        let body = Json(ErrorResponse {
            error: self.0.to_string(),
        });
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

// -- Helpers --

fn map_invoice(r: &LookupInvoiceResponse) -> InvoiceResponse {
    InvoiceResponse {
        transaction_type: r.transaction_type.as_ref().map(|t| match t {
            TransactionType::Incoming => "incoming".to_string(),
            TransactionType::Outgoing => "outgoing".to_string(),
        }),
        state: r.state.as_ref().map(|s| match s {
            TransactionState::Pending => "pending",
            TransactionState::Settled => "settled",
            TransactionState::Expired => "expired",
            TransactionState::Failed => "failed",
            TransactionState::Accepted => "accepted",
        }.to_string()),
        invoice: r.invoice.clone(),
        description: r.description.clone(),
        preimage: r.preimage.clone(),
        payment_hash: r.payment_hash.clone(),
        amount: r.amount,
        fees_paid: r.fees_paid,
        created_at: r.created_at.map(|t| t.as_secs()),
        expires_at: r.expires_at.map(|t| t.as_secs()),
        settled_at: r.settled_at.map(|t| t.as_secs()),
        metadata: r.metadata.clone(),
    }
}

fn transaction_type_str(t: &TransactionType) -> String {
    match t {
        TransactionType::Incoming => "incoming".to_string(),
        TransactionType::Outgoing => "outgoing".to_string(),
    }
}

fn transaction_state_str(s: &TransactionState) -> String {
    match s {
        TransactionState::Pending => "pending".to_string(),
        TransactionState::Settled => "settled".to_string(),
        TransactionState::Expired => "expired".to_string(),
        TransactionState::Failed => "failed".to_string(),
        TransactionState::Accepted => "accepted".to_string(),
    }
}

pub fn map_notification(
    notification_type: NotificationType,
    result: NotificationResult,
) -> EventNotification {
    match result {
        NotificationResult::PaymentReceived(p) | NotificationResult::PaymentSent(p) => {
            EventNotification {
                notification_type: notification_type.to_string(),
                transaction_type: p.transaction_type.as_ref().map(transaction_type_str),
                state: p.state.as_ref().map(transaction_state_str),
                invoice: Some(p.invoice),
                description: p.description,
                preimage: Some(p.preimage),
                payment_hash: Some(p.payment_hash),
                amount: Some(p.amount),
                fees_paid: Some(p.fees_paid),
                created_at: p.created_at.map(|t| t.as_secs()),
                expires_at: p.expires_at.map(|t| t.as_secs()),
                settled_at: Some(p.settled_at.as_secs()),
                settle_deadline: None,
                metadata: p.metadata,
            }
        }
        NotificationResult::HoldInvoiceAccepted(h) => EventNotification {
            notification_type: notification_type.to_string(),
            transaction_type: Some(transaction_type_str(&h.transaction_type)),
            state: h.state.as_ref().map(transaction_state_str),
            invoice: Some(h.invoice),
            description: h.description,
            preimage: None,
            payment_hash: Some(h.payment_hash),
            amount: Some(h.amount),
            fees_paid: None,
            created_at: Some(h.created_at.as_secs()),
            expires_at: Some(h.expires_at.as_secs()),
            settled_at: None,
            settle_deadline: Some(h.settle_deadline),
            metadata: h.metadata,
        },
    }
}

// -- Handlers --

pub async fn get_info(
    State(state): State<Arc<AppState>>,
) -> Result<Json<InfoResponse>, AppError> {
    let info = state.nwc.get_info().await?;
    Ok(Json(InfoResponse {
        alias: info.alias,
        color: info.color,
        pubkey: info.pubkey,
        network: info.network,
        block_height: info.block_height,
        block_hash: info.block_hash,
        methods: info.methods.iter().map(|m| m.to_string()).collect(),
        notifications: info.notifications,
    }))
}

pub async fn get_balance(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BalanceResponse>, AppError> {
    let balance_msats = state.nwc.get_balance().await?;
    Ok(Json(BalanceResponse {
        balance_msats,
        balance_sats: balance_msats / 1000,
    }))
}

pub async fn pay_invoice(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PayRequest>,
) -> Result<Json<PayResponse>, AppError> {
    let res = state
        .nwc
        .pay_invoice(PayInvoiceRequest {
            id: None,
            invoice: req.invoice,
            amount: req.amount,
        })
        .await?;
    Ok(Json(PayResponse {
        preimage: res.preimage,
        fees_paid: res.fees_paid,
    }))
}

pub async fn create_invoice(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateInvoiceRequest>,
) -> Result<Json<CreateInvoiceResponse>, AppError> {
    let res = state
        .nwc
        .make_invoice(MakeInvoiceRequest {
            amount: req.amount_sats * 1000,
            description: req.description,
            description_hash: req.description_hash,
            expiry: req.expiry,
        })
        .await?;
    Ok(Json(CreateInvoiceResponse {
        invoice: res.invoice,
        payment_hash: res.payment_hash,
        expires_at: res.expires_at.map(|t| t.as_secs()),
    }))
}

pub async fn lookup_invoice(
    State(state): State<Arc<AppState>>,
    Path(payment_hash): Path<String>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let res = state
        .nwc
        .lookup_invoice(LookupInvoiceRequest {
            payment_hash: Some(payment_hash),
            invoice: None,
        })
        .await?;
    Ok(Json(map_invoice(&res)))
}

pub async fn list_transactions(
    State(state): State<Arc<AppState>>,
    Query(q): Query<TransactionQuery>,
) -> Result<Json<TransactionsResponse>, AppError> {
    let transaction_type = q.transaction_type.as_deref().map(|t| match t {
        "incoming" => TransactionType::Incoming,
        "outgoing" => TransactionType::Outgoing,
        _ => TransactionType::Incoming,
    });

    let res = state
        .nwc
        .list_transactions(ListTransactionsRequest {
            from: None,
            until: None,
            limit: q.limit,
            offset: q.offset,
            unpaid: q.unpaid,
            transaction_type,
            payment_method: None,
        })
        .await?;

    Ok(Json(TransactionsResponse {
        transactions: res.iter().map(map_invoice).collect(),
    }))
}

pub async fn sse_events(
    State(state): State<Arc<AppState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.events_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(notification) => {
            let data = serde_json::to_string(&notification).ok()?;
            Some(Ok(Event::default()
                .event(&notification.notification_type)
                .data(data)))
        }
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
