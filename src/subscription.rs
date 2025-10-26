use iced::futures::{StreamExt, stream::unfold};
use std::{collections, hash::Hasher, sync};

use crate::*;

pub(crate) struct VisibilityUpdater {
	pub(crate) id: usize,
	pub(crate) frame_tracker: sync::Arc<sync::Mutex<collections::BTreeMap<usize, [bool; 2]>>>,
}

impl iced::advanced::subscription::Recipe for VisibilityUpdater {
	type Output = message::IcedWryMessage;

	fn hash(
		&self,
		state: &mut iced::advanced::subscription::Hasher,
	) {
		state.write(b"iced_wry::subscription::VisibilityUpdater");
		state.write_usize(self.id);
	}

	fn stream(
		self: Box<Self>,
		input: iced::advanced::subscription::EventStream,
	) -> iced::advanced::graphics::futures::BoxStream<Self::Output> {
		let stream = unfold((self.frame_tracker.clone(), input), |(state, mut event_stream)| async move {
			let output = loop {
				if let Some(iced::advanced::subscription::Event::Interaction {
					event: iced::Event::Window(iced::window::Event::RedrawRequested(_)),
					..
				}) = event_stream.next().await
				{
					let lock = state.lock().unwrap();

					if lock.is_empty() {
						continue;
					} else {
						break lock.iter().filter(|(_, frame)| matches!(frame, [true, false])).map(|(id, _)| *id).collect::<Vec<_>>();
					};
				}
			};

			Some((message::IcedWryMessage::HideWebviews(output), (state, event_stream)))
		});

		Box::pin(stream)
	}
}
