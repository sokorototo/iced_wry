use iced::widget;
use std::time;

#[derive(Debug, Clone)]
enum Message {
	EditUrlInput(String),
	CreateView,
	ToggleWebview,
	CreatedMainWindow(iced::window::Id),
	ExtractedWindowHandle(iced_wry::ExtractedWindowId),
	IcedWryMessage(iced_wry::IcedWryMessage),
}

struct State {
	url_input: String,

	main_window: Option<iced::window::Id>,
	webview_manager: iced_wry::IcedWebviewManager,
	webview: Option<iced_wry::IcedWebview>,
	webview_visible: bool,
}

fn main() {
	fn new() -> (State, iced::Task<Message>) {
		let open_window_task = iced::window::open(iced::window::Settings {
			size: iced::Size { width: 800.0, height: 800.0 },
			..Default::default()
		});

		(
			State {
				url_input: Default::default(),

				main_window: None,
				webview_manager: iced_wry::IcedWebviewManager::new(),
				webview: Default::default(),
				webview_visible: true,
			},
			open_window_task.1.map(Message::CreatedMainWindow),
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
			Message::CreateView => return iced_wry::IcedWebviewManager::extract_window_id(state.main_window).map(Message::ExtractedWindowHandle),
			Message::ToggleWebview => state.webview_visible = !state.webview_visible,
			Message::ExtractedWindowHandle(id) => {
				let mut attributes = iced_wry::wry::WebViewAttributes::default();
				attributes.url = Some(state.url_input.clone());

				let webview = state.webview_manager.new_webview(attributes, id).unwrap();
				state.webview = Some(webview);
			}
			Message::CreatedMainWindow(id) => state.main_window = Some(id),
			Message::IcedWryMessage(msg) => state.webview_manager.update(msg),
		}

		iced::Task::none()
	}

	fn view<'a>(
		state: &'a State,
		_: iced::window::Id,
	) -> widget::Column<'a, Message> {
		widget::column![widget::row![
			widget::text_input("Enter a URL to open", state.url_input.as_str()).on_input(Message::EditUrlInput),
			widget::button("Go To").on_press_maybe((!state.url_input.is_empty()).then_some(Message::CreateView)),
			widget::button(if state.webview_visible && state.webview.is_some() { "Hide Webview" } else { "Show Webview" }).on_press(Message::ToggleWebview),
		]]
		.push_maybe(state.webview_visible.then(|| state.webview.as_ref().map(|w| w.view(iced::Length::Fill, iced::Length::Fill))).flatten())
		.push_maybe((!state.webview_visible).then_some(widget::text("Webview Not Displayed :)")))
	}

	fn subscription<'a>(state: &'a State) -> iced::Subscription<Message> {
		state.webview_manager.subscription(time::Duration::from_millis(25)).map(Message::IcedWryMessage)
	}

	iced::daemon::<_, Message, iced::Theme, iced::Renderer>("Simple Webview Test", update, view)
		.subscription(subscription)
		.run_with(new)
		.unwrap();
}
