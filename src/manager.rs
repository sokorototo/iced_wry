use crate::message::WebviewUpdateMessage;
use std::{sync::{mpsc, self}, cell};

static UPDATER: sync::OnceLock<mpsc::Sender<(usize, WebviewUpdateMessage)>> = sync::OnceLock::new();

pub(crate) fn get_updater() -> mpsc::Sender<(usize, WebviewUpdateMessage)> {
	let (sender, receiver) = mpsc::channel();

	// TODO: Implement webview manager

	sender
}

pub(crate) fn get_initializer() -> mpsc::Sender<(usize, (wry::WebView, oneshot::Sender<()>))> {
	let (sender, receiver) = mpsc::channel();

	let (p, (pp, wv)) = receiver.try_recv().unwrap();

	// TODO: Implement webview manager

	sender
}
