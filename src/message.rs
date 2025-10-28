/// Messages to be synced with [`update`](crate::IcedWebviewManager::update)
#[derive(Debug, Clone)]
pub enum IcedWryMessage {
	/// Webviews that haven't been displayed in a specific duration automatically get hidden. This is usually because the webview wasn't included in the widget tree returned by [`view`](iced::application)
	HideWebviews(Vec<usize>),
}
