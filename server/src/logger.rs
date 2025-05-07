use tower_lsp::lsp_types::MessageType;

static LOGGER: once_cell::sync::OnceCell<tokio::sync::broadcast::Sender<(MessageType, String)>> =
    once_cell::sync::OnceCell::new();

pub fn log(tye: MessageType, message: String) {
    if let Some(tx) = LOGGER.get() {
        let _ = tx.send((tye, message));
    }
}

pub fn subscribe() -> tokio::sync::broadcast::Receiver<(MessageType, String)> {
    LOGGER
        .get_or_init(|| {
            let (tx, _) = tokio::sync::broadcast::channel(100);
            tx
        })
        .subscribe()
}
