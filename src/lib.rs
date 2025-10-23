use std::{collections, rc, sync, sync::mpsc};
pub use wry;

mod manager;
mod message;

/// Stores state for synchronizing visibility and bounds for any managed [`webviews`](wry::WebView)
#[derive(Debug, Clone)]
pub struct IcedWebviewManager {
	webviews: collections::BTreeMap<usize, rc::Weak<wry::WebView>>,
	updater: mpsc::Sender<(usize, message::WebviewUpdateMessage)>,
}

impl IcedWebviewManager {
	pub(crate) fn increment_id() -> usize {
		pub static WEBVIEW_COUNTER: sync::atomic::AtomicUsize = sync::atomic::AtomicUsize::new(1);
		WEBVIEW_COUNTER.fetch_add(1, sync::atomic::Ordering::Relaxed)
	}

	/// Use [`get_oldest`](iced::window::get_oldest) to acquire the main window's [`Id`](iced::window::Id), then use [`run_with_handle`](iced::window::run_with_handle) to acquire the [`WindowHandle`](iced::window::raw_window_handle::WindowHandle)
	pub fn new_webview(
		&mut self,
		mut attrs: wry::WebViewAttributes<'static>,
		window_handle: iced::window::raw_window_handle::WindowHandle<'_>,
	) -> Result<IcedWebview, wry::Error> {
		let id = IcedWebviewManager::increment_id();
		attrs.visible = false;

		// persist webview state
		let webview = wry::WebView::new_as_child(&window_handle, attrs)?;
		let inner = rc::Rc::new(webview);

		self.webviews.insert(id, rc::Rc::downgrade(&inner));

		Ok(IcedWebview {
			inner,
			id,
			updater: self.updater.clone(),
		})
	}
}

#[derive(Clone)]
pub struct IcedWebview {
	inner: rc::Rc<wry::WebView>,
	id: usize,
	updater: mpsc::Sender<(usize, message::WebviewUpdateMessage)>,
}

impl IcedWebview {
	/// Acquire a [`IcedWebviewContainerElement`] for layout and rendering the [`webview`](wry::WebView) overlay
	pub fn view(
		&self,
		width: impl Into<iced::Length>,
		height: impl Into<iced::Length>,
	) -> IcedWebviewContainerElement {
		IcedWebviewContainerElement {
			inner: self.clone(),
			width: width.into(),
			height: height.into(),
		}
	}
}

impl AsRef<wry::WebView> for IcedWebview {
	fn as_ref(&self) -> &wry::WebView {
		&self.inner
	}
}

pub struct IcedWebviewContainerElement {
	inner: IcedWebview,
	width: iced::Length,
	height: iced::Length,
}

impl<Message, Theme, R: iced::advanced::Renderer> iced::advanced::Widget<Message, Theme, R> for IcedWebviewContainerElement {
	fn size(&self) -> iced::Size<iced::Length> {
		iced::Size {
			width: self.width,
			height: self.height,
		}
	}

	fn layout(
		&self,
		_tree: &mut iced::advanced::widget::Tree,
		_renderer: &R,
		limits: &iced::advanced::layout::Limits,
	) -> iced::advanced::layout::Node {
		let node = iced::advanced::layout::Node::new(limits.resolve(self.width, self.height, iced::Size::ZERO));
		let bounds = node.bounds();

		let rect = wry::Rect {
			position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(bounds.x.into(), bounds.x.into())),
			size: wry::dpi::LogicalSize::<f64>::new(bounds.width.into(), bounds.height.into()).into(),
		};

		if let Err(err) = self.inner.as_ref().set_bounds(rect) {
			eprintln!("Unable to set bounds for webview with id: {}\n{}", self.inner.id, err)
		};

		node
	}

	fn draw(
		&self,
		_tree: &iced::advanced::widget::Tree,
		_renderer: &mut R,
		_theme: &Theme,
		_style: &iced::advanced::renderer::Style,
		_layout: iced::advanced::Layout<'_>,
		_cursor: iced::advanced::mouse::Cursor,
		_viewport: &iced::Rectangle,
	) {
		// webview is rendered as an overlay, the overlay is
		self.inner.updater.send((self.inner.id, message::WebviewUpdateMessage::PersistFrame)).unwrap()
	}
}

impl Drop for IcedWebviewContainerElement {
	fn drop(&mut self) {
		self.inner.updater.send((self.inner.id, message::WebviewUpdateMessage::DecayFrame)).unwrap()
	}
}
