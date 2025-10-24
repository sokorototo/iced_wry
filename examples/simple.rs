use iced::widget;

#[derive(Debug, Clone)]
enum Message {
	EditUrlInput(String),
	CreateView,
	ToggleWebview,
	CreatedMainWindow(iced::window::Id),
	UnsafeExtractedWindowHandle(usize),
}

struct State {
	frames: usize,
	url_input: String,

	main_window: Option<iced::window::Id>,
	webview_manager: iced_wry::IcedWebviewManager,
	webview: Option<iced_wry::IcedWebview>,
	webview_visible: bool,
}

fn main() {
	fn new() -> (State, iced::Task<Message>) {
		let open_window_task = iced::window::open(iced::window::Settings { ..Default::default() }).1;
		(
			State {
				frames: 0,
				url_input: Default::default(),

				main_window: None,
				webview_manager: iced_wry::IcedWebviewManager::new(),
				webview: Default::default(),
				webview_visible: true,
			},
			open_window_task.map(Message::CreatedMainWindow),
		)
	}

	fn update(
		state: &mut State,
		message: Message,
	) -> iced::Task<Message> {
		match message {
			Message::EditUrlInput(chars) => {
				state.url_input = chars;
			}
			Message::CreateView => {
				if let Some(id) = state.main_window {
					// create webview
					return iced::window::run_with_handle(id, |handle| {
						let raw = handle.as_raw();
						Box::into_raw(Box::new(raw)) as usize
					})
					.map(Message::UnsafeExtractedWindowHandle);
				}
			}
			Message::ToggleWebview => state.webview_visible = !state.webview_visible,
			Message::UnsafeExtractedWindowHandle(ptr) => {
				let raw = unsafe {
					let ptr = ptr as *mut iced::window::raw_window_handle::RawWindowHandle;
					Box::from_raw(ptr)
				};
				let window_handle = unsafe { iced::window::raw_window_handle::WindowHandle::borrow_raw(*raw) };

				let mut attributes = iced_wry::wry::WebViewAttributes::default();
				attributes.url = Some(state.url_input.clone());

				let webview = state.webview_manager.new_webview(attributes, &window_handle).unwrap();
				state.webview = Some(webview);
			}
			Message::CreatedMainWindow(id) => state.main_window = Some(id),
		}

		iced::Task::none()
	}

	fn view<'a>(
		state: &'a State,
		_: iced::window::Id,
	) -> widget::Column<'a, Message> {
		println!("`view` called");

		widget::column![
			widget::row![
				widget::text!("Frames Rendered: {}", state.frames),
				widget::Space::with_width(iced::Length::Fill),
				widget::button("Toggle Visibility").on_press(Message::ToggleWebview)
			],
			widget::row![
				widget::text_input("Enter a URL to open", state.url_input.as_str()).on_input(Message::EditUrlInput),
				widget::button("Go To").on_press(Message::CreateView),
				widget::Space::with_height(20)
			]
		]
		.push_maybe(state.webview_visible.then(|| state.webview.as_ref().map(|w| w.view(iced::Length::Fill, iced::Length::Fill))).flatten())
	}

	iced::daemon::<_, Message, iced::Theme, iced::Renderer>("Simple Webview Test", update, view).run_with(new).unwrap();
}
