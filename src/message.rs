pub enum IcedWryMessage {
	WebviewInitialized(super::IcedWebview),
	WryError(wry::Error),
}

pub(crate) enum WebviewUpdateMessage {
	CreateWebview,
	// UI
	SetVisible(bool),
	SetBounds(iced::Rectangle),
	SetBackgroundColor(u8, u8, u8, u8),
	//devtools
	OpenDevTools,
	CloseDevTools,
	// page loading
	Reload,
	LoadHtml(String),
	LoadUrl(String),
	// frame management
	PersistFrame,
	DecayFrame,
}
