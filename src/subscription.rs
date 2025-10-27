use iced::futures::{StreamExt, stream::unfold};
use std::{collections, hash::Hasher, sync, time};

use crate::*;

pub(crate) struct VisibilityUpdater {
	pub(crate) persist_duration: time::Duration,
	pub(crate) id: usize,
	pub(crate) frame_tracker: sync::Arc<sync::Mutex<collections::BTreeMap<usize, time::Instant>>>,
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
		let stream = unfold((self, input), |(state, mut event_stream)| async move {
			let output = loop {
				if let Some(iced::advanced::subscription::Event::Interaction {
					event: iced::Event::Window(iced::window::Event::RedrawRequested(now)),
					..
				}) = event_stream.next().await
				{
					let tracker = state.frame_tracker.lock().unwrap();

					if tracker.is_empty() {
						continue;
					} else {
						let hidden = tracker
							.iter()
							.filter(|(_, last_rendered)| (now - **last_rendered) >= state.persist_duration)
							.map(|(id, _)| *id)
							.collect::<Vec<_>>();

						if !hidden.is_empty() {
							break hidden;
						}
					};
				}
			};

			Some((message::IcedWryMessage::HideWebviews(output), (state, event_stream)))
		});

		Box::pin(stream)
	}
}
