#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

mod message;
mod subscription;

pub use message::IcedWryMessage;
use std::{cell, collections, sync, time};
pub use wry;

thread_local! {
	static WINDOW_HANDLES: cell::RefCell<collections::BTreeMap<iced::window::Id, iced::window::raw_window_handle::RawWindowHandle>> = cell::RefCell::new(collections::BTreeMap::new());
}

/// Stores state for synchronizing visibility and bounds for any managed [`webviews`](wry::WebView)
pub struct IcedWebviewManager {
	// simply used to differentiate between subscriptions
	manager_id: usize,
	webviews: collections::BTreeMap<usize, sync::Arc<wry::WebView>>,
	// tracks the last moment a webview was rendered, and hides it if the instant has past by a set duration
	display_tracker: sync::Arc<sync::Mutex<collections::BTreeMap<usize, time::Instant>>>,
	subscription_ctl: sync::Arc<sync::Mutex<bool>>,
}

/// Wraps a [`Id`](iced::window::Id), this is necessary as `iced_wry` needs to extract the [`WindowHandle`](iced::window::raw_window_handle) from the iced runtime
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ExtractedWindowId(iced::window::Id);

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
			subscription_ctl: sync::Arc::new(sync::Mutex::new(true)),
		}
	}

	/// Pass [`None`] to use the main window. If no window is active, Task never yields
	pub fn extract_window_id(window_id: Option<iced::window::Id>) -> iced::Task<ExtractedWindowId> {
		if let Some(id) = window_id {
			if WINDOW_HANDLES.with_borrow_mut(move |handles| handles.contains_key(&id)) {
				return iced::Task::done(ExtractedWindowId(id));
			};
		}

		match window_id {
			Some(id) => iced::window::run_with_handle(id, move |handle| {
				let raw = handle.as_raw();

				WINDOW_HANDLES.with_borrow_mut(move |handles| {
					let _ = handles.insert(id, raw);
				});

				ExtractedWindowId(id)
			}),
			None => iced::window::get_oldest().then(move |id| match id {
				Some(id) => iced::window::run_with_handle(id, move |handle| {
					let raw = handle.as_raw();

					WINDOW_HANDLES.with_borrow_mut(move |handles| {
						let _ = handles.insert(id, raw);
					});

					ExtractedWindowId(id)
				}),
				None => iced::Task::none(),
			}),
		}
	}

	/// Use the [`usize`] yielded by [`extract_window_id`](IcedWebviewManager::extract_window_id) to spawn a webview
	pub fn new_webview(
		&mut self,
		mut attrs: wry::WebViewAttributes<'static>,
		window_id: ExtractedWindowId,
	) -> Option<IcedWebview> {
		attrs.visible = false;
		attrs.focused = false;

		// extract the window handle
		let result = WINDOW_HANDLES.with_borrow_mut(move |w| {
			w.get(&window_id.0).map(|raw| {
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

		self.webviews.insert(webview_id, sync::Arc::clone(&webview));

		Some(IcedWebview {
			webview: sync::Arc::downgrade(&webview),
			id: webview_id,
			tracker: self.display_tracker.clone(),
		})
	}

	/// Subscription that runs every frame and automatically hides webviews that haven't been rendered for [`persist_duration`]
	pub fn subscription(
		&self,
		persist_duration: time::Duration,
	) -> iced::Subscription<message::IcedWryMessage> {
		let tracker = self.display_tracker.clone();
		let recipe = subscription::VisibilityUpdater {
			persist_duration,
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
					if let Some(webview) = self.webviews.get(&id) {
						if let Err(err) = webview.set_visible(false) {
							eprintln!("Unable to update visibility for webview with id: {}\n{}", id, err)
						};
					} else {
						eprintln!("Unable to find webview with id: {}", id)
					}
				}
			}
		}
	}

	/// Completely resets the manager's internal state
	pub fn reset(&mut self) {
		self.webviews.clear();
		if let Ok(mut tracker) = self.display_tracker.lock() {
			tracker.clear();
		}
	}
}

impl Drop for IcedWebviewManager {
	fn drop(&mut self) {
		//set abort controller to false
		let mut ctl = self.subscription_ctl.lock().unwrap();
		*ctl = false;
	}
}

/// Contains state necessary for layout and display of a specific webview
pub struct IcedWebview {
	webview: sync::Weak<wry::WebView>,
	tracker: sync::Arc<sync::Mutex<collections::BTreeMap<usize, time::Instant>>>,
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
		if let Some(webview) = sync::Weak::upgrade(&self.inner.webview) {
			let bounds = layout.bounds();
			let rect = wry::Rect {
				position: wry::dpi::Position::Logical(wry::dpi::LogicalPosition::new(bounds.x.into(), bounds.y.into())),
				size: wry::dpi::LogicalSize::<f64>::new(bounds.width.into(), bounds.height.into()).into(),
			};

			// update overlay state
			if let Err(err) = webview.set_bounds(rect) {
				eprintln!("Unable to set bounds for webview with id: {}\n{}", self.inner.id, err)
			};

			if let Err(err) = webview.set_visible(true) {
				eprintln!("Unable to update visibility for webview with id: {}\n{}", self.inner.id, err)
			};
		} else {
			eprintln!("Attempted to render webview, when WebviewManager was already dropped")
		};
	}

	fn on_event(
		&mut self,
		_state: &mut iced::advanced::widget::Tree,
		event: iced::Event,
		layout: iced::advanced::Layout<'_>,
		cursor: iced::advanced::mouse::Cursor,
		_renderer: &R,
		_clipboard: &mut dyn iced::advanced::Clipboard,
		_shell: &mut iced::advanced::Shell<'_, Message>,
		_viewport: &iced::Rectangle,
	) -> iced::advanced::graphics::core::event::Status {
		let instant = match event {
			iced::Event::Window(iced::window::Event::RedrawRequested(instant)) => instant,
			iced::Event::Mouse(iced::mouse::Event::ButtonPressed(..)) => {
				let bounds = layout.bounds();
				if let Some(pos) = cursor.position() {
					if !bounds.contains(pos) {
						if let Some(webview) = sync::Weak::upgrade(&self.inner.webview) {
							if let Err(err) = webview.focus_parent() {
								eprintln!("Unable to focus parent for webview with id: {}\n{}", self.inner.id, err)
							};
						}
					}
				};

				return iced::advanced::graphics::core::event::Status::Ignored;
			}
			_ => {
				return iced::advanced::graphics::core::event::Status::Ignored;
			}
		};

		if let Ok(mut guard) = self.inner.tracker.lock() {
			guard
				.entry(self.inner.id)
				.and_modify(|s| {
					*s = instant;
				})
				.or_insert(instant);
		} else {
			eprintln!("Unable to acquire lock for internal Arc<Mutex> tracker")
		};

		iced::advanced::graphics::core::event::Status::Ignored
	}
}

impl<'a> Drop for IcedWebviewContainerElement<'a> {
	fn drop(&mut self) {
		if let Some(webview) = sync::Weak::upgrade(&self.inner.webview) {
			if let Err(err) = webview.focus_parent() {
				eprintln!("Unable to focus parent for webview with id: {}\n{}", self.inner.id, err)
			};
		}
	}
}
