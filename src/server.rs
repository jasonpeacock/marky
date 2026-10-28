use std::{cell::RefCell, future::IntoFuture, net::SocketAddr, path::PathBuf};

use crate::document;
use crate::{error, info, service, warn};
use axum::{routing::get, Extension, Router};
use colored::Colorize;
use tokio::sync::{
    oneshot,
    watch::{self, Sender},
};

#[derive(Clone)]
pub struct Config {
    pub root_dir: PathBuf,
    pub render_options: document::RenderOptions,
}

/// Code is taken from the https://github.com/euclio/aurelius/

/// Markdown preview server.
///
/// Listens for HTTP connections and serves a page containing a live markdown preview. The page
/// contains JavaScript to open a websocket connection back to the server for rendering updates.
#[derive(Debug)]
pub struct Server {
    output: RefCell<String>,
    tx: Sender<String>,
    _shutdown_tx: oneshot::Sender<()>,
}

impl Server {
    pub fn bind(addr: &SocketAddr, config: Config) -> anyhow::Result<Self> {
        let (tx, rx) = watch::channel(String::new());
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        let app = Router::new()
            .route("/", get(service::websocket_handler))
            .fallback_service(get(service::serve_static_file))
            .layer(Extension(rx))
            .layer(Extension(config));

        // Bind synchronously so a busy port is reported as an error to the caller.
        let listener = std::net::TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let listener = tokio::net::TcpListener::from_std(listener)?;
        let addr = listener.local_addr()?;

        info!("Listening on {}", addr);
        info!("Opening in browser");

        if let Err(e) = open::that(format!("http://{}", addr)) {
            warn!("Failed to open the page: {}", e);
        }

        let http_server = axum::serve(listener, app).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });

        tokio::spawn(http_server.into_future());

        Ok(Server {
            output: RefCell::new(String::new()),
            tx,
            _shutdown_tx: shutdown_tx,
        })
    }

    /// Pushes the rendered document to connected browsers and returns whether
    /// rendering succeeded. On failure they keep showing the last good render.
    pub async fn send(&self, document: &crate::document::Document) -> bool {
        match document.render_body() {
            Ok(body) => {
                self.output.replace(self.tx.send_replace(body));
                true
            }
            Err(e) => {
                error!("render failed: {:#}", e);
                false
            }
        }
    }

    // TODO: use it
    pub async fn _shutdown(self) -> Result<(), ()> {
        self._shutdown_tx.send(())
    }
}
