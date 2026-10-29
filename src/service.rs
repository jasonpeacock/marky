use axum::{
    body::Body,
    extract::{
        ws::{
            rejection::WebSocketUpgradeRejection, Message as AxumMessage, WebSocket,
            WebSocketUpgrade,
        },
        Extension,
    },
    http::{Request, StatusCode},
    response::{Html, IntoResponse},
};
use std::time::Duration;
use tokio::sync::watch::Receiver;
use tower::util::ServiceExt;
use tower_http::services::ServeDir;

pub async fn websocket_handler(
    ws: Result<WebSocketUpgrade, WebSocketUpgradeRejection>,
    Extension(config): Extension<crate::server::Config>,
    Extension(html_rx): Extension<Receiver<String>>,
) -> impl IntoResponse {
    if let Ok(ws) = ws {
        return ws.on_upgrade(|ws| async { handle_websocket(ws, html_rx).await });
    }

    let doc = crate::document::Document {
        text: "😴 Waiting for changes".into(),
        options: config.render_options,
        base_dir: config.root_dir,
    };

    let buffer = doc.render().expect("Document with empty text must render");
    let html = String::from_utf8(buffer).expect("Must be a valid utf8");
    (StatusCode::OK, Html(html)).into_response()
}

/// How often to ping the client, so proxies and tunnels don't drop an idle connection.
const PING_INTERVAL: Duration = Duration::from_secs(30);

async fn handle_websocket(mut socket: WebSocket, mut html_rx: Receiver<String>) {
    // Bring a reconnecting page up to date without waiting for the next save.
    let current = html_rx.borrow_and_update().clone();
    if !current.is_empty() && socket.send(AxumMessage::Text(current.into())).await.is_err() {
        return;
    }

    let mut ping = tokio::time::interval(PING_INTERVAL);
    // The first tick completes immediately; skip it.
    ping.tick().await;

    loop {
        let message = tokio::select! {
            changed = html_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                AxumMessage::Text(html_rx.borrow_and_update().clone().into())
            }
            _ = ping.tick() => AxumMessage::Ping(Default::default()),
            incoming = socket.recv() => match incoming {
                // Pongs and anything else the page sends need no reply.
                Some(Ok(AxumMessage::Close(_))) | Some(Err(_)) | None => return,
                Some(Ok(_)) => continue,
            },
        };

        if socket.send(message).await.is_err() {
            // The client went away; its tab was closed or reloaded.
            return;
        }
    }

    let _ = socket.send(AxumMessage::Close(None)).await;
}

pub async fn serve_static_file(
    Extension(config): Extension<crate::server::Config>,
    req: Request<Body>,
) -> impl IntoResponse {
    let service = ServeDir::new(config.root_dir);

    service
        .oneshot(req)
        .await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}
