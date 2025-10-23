use iced::widget;

#[derive(Debug, Clone, Copy)]
enum TestMessage {}

fn main() {
	fn update(
		state: &mut u64,
		message: TestMessage,
	) {
	}

	fn view<'a>(value: &'a u64) -> widget::Column<'a, TestMessage> {
		widget::column![widget::text("Simple display cuh"), widget::Space::with_height(25), widget::text("Add a little bit of spice")]
	}

	iced::run::<u64, TestMessage, iced::Theme, iced::Renderer>("Simple Webview Test", update, view).unwrap();
}
