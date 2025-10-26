mod message;
mod subscription;

pub use message::IcedWryMessage;
use std::{cell, collections, sync};
pub use wry;

thread_local! {
	static WINDOW_HANDLES: cell::RefCell<collections::BTreeMap<usize, iced::window::raw_window_handle::RawWindowHandle>> = cell::RefCell::new(collections::BTreeMap::new());
}

/// Stores state for synchronizing visibility and bounds for any managed [`webviews`](wry::WebView)
#[derive(Debug, Clone)]
pub struct IcedWebviewManager {
	// simply used to differentiate between subscriptions
	manager_id: usize,
	webviews: collections::BTreeMap<usize, sync::Weak<wry::WebView>>,
	// tracks the last active frame when a webview was rendered, hiding any webviews where the last active frame is lower than the current active frame
	display_tracker: sync::Arc<sync::Mutex<collections::BTreeMap<usize, [bool; 2]>>>,
}

impl IcedWebviewManager {
	pub(crate) fn increment_id() -> usize {
		pub static WEBVIEW_COUNTER: sync::atomic::AtomicUsize = sync::atomic::AtomicUsize::new(1);
		WEBVIEW_COUNTER.fetch_add(1, sync::atomic::Ordering::Relaxed)
	}

	/// Instantiate a new manager cuz
	pub fn new() -> IcedWebviewManager {
		IcedWebviewManager {
			manager_id: IcedWebviewManager::increment_id(),
			webviews: collections::BTreeMap::new(),
			display_tracker: sync::Arc::new(sync::Mutex::new(collections::BTreeMap::new())),
		}
	}

	/// Pass [`None`] to use the main window. If no window is active, Task never yields
	pub fn acquire_window_handle(window_id: Option<iced::window::Id>) -> iced::Task<usize> {
		let _id = IcedWebviewManager::increment_id();

		match window_id {
			Some(id) => iced::window::run_with_handle(id, move |handle| {
				let raw = handle.as_raw();

				WINDOW_HANDLES.with_borrow_mut(move |handles| {
					let _ = handles.insert(_id, raw);
				});

				_id
			}),
			None => iced::window::get_oldest().then(move |id| match id {
				Some(id) => iced::window::run_with_handle(id, move |handle| {
					let raw = handle.as_raw();

					WINDOW_HANDLES.with_borrow_mut(move |handles| {
						let _ = handles.insert(_id, raw);
					});

					_id
				}),
				None => iced::Task::none(),
			}),
		}
	}

	/// Use the [`usize`] yielded by [`acquire_window_handle`] to spawn a webview
	pub fn new_webview(
		&mut self,
		mut attrs: wry::WebViewAttributes<'static>,
		window_id: usize,
	) -> Option<IcedWebview> {
		attrs.visible = false;
		attrs.focused = false;

		// acquire window handle
		let result = WINDOW_HANDLES.with_borrow_mut(move |w| {
			w.get(&window_id).map(|raw| {
				let window_handle = unsafe { iced::window::raw_window_handle::WindowHandle::borrow_raw(*raw) };
				wry::WebView::new_as_child(&window_handle, attrs)
			})
		})?;

		let webview = match result {
			Ok(w) => w,
			Err(e) => {
				eprintln!("Unable to create webview: {}", e);
				return None;
			}
		};

		// persist webview state
		let webview_id = IcedWebviewManager::increment_id();
		let webview = sync::Arc::new(webview);

		self.webviews.insert(webview_id, sync::Arc::downgrade(&webview));

		// setup frame persistence state
		if let Ok(mut guard) = self.display_tracker.lock() {
			guard.entry(webview_id).and_modify(|s| *s = [false, false]).or_insert([false, false]);
		};

		Some(IcedWebview {
			webview,
			id: webview_id,
			tracker: self.display_tracker.clone(),
		})
	}

	/// Subscription that runs every frame, and syncs visibility and bounds for managed overlays
	pub fn subscription(&self) -> iced::Subscription<message::IcedWryMessage> {
		let tracker = self.display_tracker.clone();
		let recipe = subscription::VisibilityUpdater {
			id: self.manager_id,
			frame_tracker: tracker,
		};

		iced::advanced::subscription::from_recipe(recipe)
	}

	/// Updates state for webviews updates sent by [`IcedWebviewManager::subscription`]
	pub fn update(
		&mut self,
		msg: message::IcedWryMessage,
	) {
		match msg {
			message::IcedWryMessage::HideWebviews(ids) => {
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

impl<'a> Drop for IcedWebviewContainerElement<'a> {
	fn drop(&mut self) {
		if let Err(err) = self.inner.webview.focus_parent() {
			eprintln!("Unable to focus parent for webview with id: {}\n{}", self.inner.id, err)
		};
	}
}
