use iced::futures::*;
use std::{collections, hash::Hasher, sync, time};

use crate::*;

pub(crate) struct VisibilityUpdater {
	pub(crate) id: usize,
	pub(crate) persist_duration: time::Duration,
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
		let debouncer = collections::BTreeSet::new();

		let stream = stream::unfold((self, input, debouncer), |(state, mut event_stream, mut debouncer)| async move {
			// contains webviews which shouldn't be rendered
			let mut expired = Vec::new();

			loop {
				if let Some(iced::advanced::subscription::Event::Interaction {
					event: iced::Event::Window(iced::window::Event::RedrawRequested(now)),
					..
				}) = event_stream.next().await
				{
					let tracker = state.frame_tracker.lock().unwrap();

					if tracker.is_empty() {
						continue;
					} else {
						for (id, last_render) in tracker.iter() {
							// if a webview is already set as hidden, avoid resending it to be hidden
							match (now - *last_render) >= state.persist_duration {
								true => {
									if debouncer.contains(id) {
										continue;
									} else {
										expired.push(*id);
										debouncer.insert(*id);
									}
								}
								false => {
									debouncer.remove(id);
								}
							}
						}

						if !expired.is_empty() {
							break;
						}
					};
				}
			}

			Some((message::IcedWryMessage::HideWebviews(expired), (state, event_stream, debouncer)))
		});

		Box::pin(stream)
	}
}
