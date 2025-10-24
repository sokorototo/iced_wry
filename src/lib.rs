use std::{collections, sync};
pub use wry;

pub enum IcedWryUpdate {
	HideWebviews(Vec<usize>),
}

/// Stores state for synchronizing visibility and bounds for any managed [`webviews`](wry::WebView)
#[derive(Debug, Clone)]
pub struct IcedWebviewManager {
	webviews: collections::BTreeMap<usize, sync::Weak<wry::WebView>>,
	// [previous, current] tracks two frame states, and only toggles off webview if state == [true, false]
	display_tracker: sync::Arc<sync::Mutex<collections::BTreeMap<usize, [bool; 2]>>>,
}

impl IcedWebviewManager {
	pub(crate) fn increment_id() -> usize {
		pub static WEBVIEW_COUNTER: sync::atomic::AtomicUsize = sync::atomic::AtomicUsize::new(1);
		WEBVIEW_COUNTER.fetch_add(1, sync::atomic::Ordering::Relaxed)
	}

	/// Initializes an iced [`subscription`](iced::Subscription) (for layout and visibility automation) and [`IcedWebviewManager`] (for Webview creation and sync).
	/// The subscription must be installed into the runtime, to keep the webview's visibility updated
	pub fn init() -> IcedWebviewManager {
		IcedWebviewManager {
			webviews: collections::BTreeMap::new(),
			display_tracker: sync::Arc::new(sync::Mutex::new(collections::BTreeMap::new())),
		}
	}

	/// Use [`get_oldest`](iced::window::get_oldest) to acquire the main window's [`Id`](iced::window::Id), then use [`run_with_handle`](iced::window::run_with_handle) to acquire the [`WindowHandle`](iced::window::raw_window_handle::WindowHandle)
	pub fn new_webview(
		&mut self,
		mut attrs: wry::WebViewAttributes<'static>,
		window_handle: &iced::window::raw_window_handle::WindowHandle<'_>,
	) -> Result<IcedWebview, wry::Error> {
		let id = IcedWebviewManager::increment_id();
		attrs.visible = false;

		// persist webview state
		let webview = wry::WebView::new_as_child(window_handle, attrs)?;
		let webview = sync::Arc::new(webview);

		self.webviews.insert(id, sync::Arc::downgrade(&webview));

		// setup frame persistence state
		if let Ok(mut guard) = self.display_tracker.lock() {
			guard.entry(id).and_modify(|s| *s = [false, false]).or_insert([false, false]);
		};

		Ok(IcedWebview {
			webview,
			id,
			tracker: self.display_tracker.clone(),
		})
	}

	/// Updates state for webviews tracked by [`display_tracker_task`](IcedWebviewManager::display_tracker_task)
	pub fn update(
		&mut self,
		update: IcedWryUpdate,
	) {
		match update {
			IcedWryUpdate::HideWebviews(ids) => {
				for id in ids {
					if let Some(weak) = self.webviews.get(&id) {
						if let Some(webview) = weak.upgrade() {
							if let Err(err) = webview.set_visible(false) {
								eprintln!("Unable to update visibility for webview with id: {}\n{}", id, err)
							};
						}
					} else {
						eprintln!("Unable to find webview with id: {}", id)
					}
				}
			}
		}
	}

	/// Task that checks state for current webviews and syncs the state
	pub fn webview_tracker_task(&self) -> iced::Task<IcedWryUpdate> {
		unimplemented!()
	}
}

pub struct IcedWebview {
	webview: sync::Arc<wry::WebView>,
	tracker: sync::Arc<sync::Mutex<collections::BTreeMap<usize, [bool; 2]>>>,
	id: usize,
}

impl IcedWebview {
	/// Acquire a [`Element`](iced::Element) for layout and rendering the [`webview`](wry::WebView) overlay
	pub fn view<'a, Message, Theme>(
		&'a self,
		width: impl Into<iced::Length>,
		height: impl Into<iced::Length>,
	) -> iced::Element<'a, Message, Theme> {
		let inner = IcedWebviewContainerElement {
			inner: self,
			width: width.into(),
			height: height.into(),
		};

		iced::Element::new(inner)
	}
}

impl AsRef<wry::WebView> for IcedWebview {
	fn as_ref(&self) -> &wry::WebView {
		&self.webview
	}
}

pub(crate) struct IcedWebviewContainerElement<'a> {
	inner: &'a IcedWebview,
	width: iced::Length,
	height: iced::Length,
}

impl<'a, Message, Theme, R: iced::advanced::Renderer> iced::advanced::Widget<Message, Theme, R> for IcedWebviewContainerElement<'a> {
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
		iced::advanced::layout::atomic(limits, self.width, self.height)
	}

	fn draw(
		&self,
		_tree: &iced::advanced::widget::Tree,
		_renderer: &mut R,
		_theme: &Theme,
		_style: &iced::advanced::renderer::Style,
		layout: iced::advanced::Layout<'_>,
		_cursor: iced::advanced::mouse::Cursor,
		_viewport: &iced::Rectangle,
	) {
		let bounds = layout.bounds();

		let rect = wry::Rect {
			position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(bounds.x.into(), bounds.y.into())),
			size: wry::dpi::LogicalSize::<f64>::new(bounds.width.into(), bounds.height.into()).into(),
		};

		if let Err(err) = self.inner.as_ref().set_bounds(rect) {
			eprintln!("Unable to set bounds for webview with id: {}\n{}", self.inner.id, err)
		};

		if let Ok(mut guard) = self.inner.tracker.lock() {
			guard
				.entry(self.inner.id)
				.and_modify(|s| {
					s.swap(0, 1);
					s[1] = true;
				})
				.or_insert([false, true]);
		} else {
			eprintln!("Unable to acquire lock for internal Arc<Mutex> tracker")
		};

		if let Err(err) = self.inner.as_ref().set_visible(true) {
			eprintln!("Unable to update visibility for webview with id: {}\n{}", self.inner.id, err)
		};
	}
}
