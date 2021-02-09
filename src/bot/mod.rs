mod message;

use std::{
	cell::RefCell,
	collections::HashSet,
	fmt::{self, Debug},
	iter::FromIterator
};

use futures::{Stream, StreamExt};

use telegram_bot::{
	Api,
	CanLeaveChat,
	CanSendMessage,
	Channel,
	MessageChat,
	MessageOrChannelPost,
	UpdateKind,
};
pub use telegram_bot::{
	ChatId,
	Error as BotError
};

pub use message::Message;
use crate::{
	config,
	db::{
		Db,
		Error as DbError,
		models::trusted_chats::NewTrustedChat,
		types::ChatId as DbChatId,
	}
};


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ChatInfo<'a> {
	chat_id: ChatId,
	username: Option<&'a str>,
	title: Option<&'a str>,
}


impl<'a> ChatInfo<'a> {
	pub fn from_update(update_kind: &'a UpdateKind) -> Option<Self> {
		match update_kind {
			UpdateKind::Message(message)  => Some(Self::from(&message.chat)),
			UpdateKind::ChannelPost(post) => Some(Self::from(&post.chat)),
			_ => None,
		}
	}
}


impl<'a> From<&'a MessageChat> for ChatInfo<'a> {
	fn from(chat: &'a MessageChat) -> Self {
		let (username, title) =  match chat {
			MessageChat::Private(user) => (
				user.username.as_deref(),
				Some(
					user.first_name.as_ref()
				)
			),
			MessageChat::Group(group) => (
				None,
				Some(
					group.title.as_ref()
				)
			),
			MessageChat::Supergroup(supergroup) => (
				supergroup.username.as_deref(),
				Some(
					supergroup.title.as_ref()
				)
			),
			MessageChat::Unknown(raw) => (
				raw.username.as_deref(),
				raw.title.as_deref()
			)
		};

		Self {
			chat_id: chat.id(),
			username,
			title
		}
	}
}


impl<'a> From<&'a Channel> for ChatInfo<'a> {
	fn from(channel: &'a Channel) -> Self {
		Self {
			chat_id: channel.id.into(),
			username: channel.username.as_deref(),
			title: Some(
				channel.title.as_ref()
			)
		}
	}
}


#[derive(Clone)]
pub struct Bot<'a> {
	api: Api,
	db: &'a Db,
	username: &'a str,
	authentication: &'a config::Authentication,
	// This needs to be a refcell so we can mutate it while streaming. Just be sure that it
	// won't be borrowed outside of the `validate_chat` method, and all shall be fine.
	trusted_chats: RefCell<HashSet<ChatId>>,
}


impl<'a> Bot<'a> {
	pub fn new<K>(
		key: K,
		db: &'a Db,
		username: &'a str,
		authentication: &'a config::Authentication
	) -> Result<Self, DbError>
	where
		K: AsRef<str>,
	{
		let key = key.as_ref();

		let trusted_chats = db.trusted_chat_ids()?;

		let trusted_chats = HashSet::from_iter(
			trusted_chats
				.iter()
				.map(
					|chat_id| chat_id.0
				)
		);

		Ok(
			Self {
				api: Api::new(key),
				db,
				username,
				authentication,
				trusted_chats: RefCell::new(trusted_chats)
			}
		)
	}


	pub fn stream(&'a self) -> impl Stream<Item = Message> + 'a {
		self.api
			.stream()
			.filter_map(
				|update| async move {
					update
						.map_err(
							|error| log::error!("Bot update error: {}", error)
						)
						.ok()
				}
			)
			.filter_map(
				move |update| async move {
					self
						.validate_chat(&update.kind)
						.await
						.then(|| update)
				}
			)
			.filter_map(
				move |update| async move {
					match Message::from_update(self.username, update.kind) {
						Ok(message) => {
							message.log();
							Some(message)
						},
						Err(error) => {
							if !error.is_unrelated() {
								error.log();
							}
							None
						}
					}
				}
			)
	}


	pub async fn leave_chat(&self, chat_id: ChatId) {
		let result = self.api
			.send(
				chat_id.leave()
			)
			.await;

		if let Err(error) = result {
			log::warn!("Failed to leave chat {}: {}", chat_id, error);
		}
	}


	pub async fn send_message<T>(
		&self,
		chat: ChatId,
		text: T,
	) -> Result<MessageOrChannelPost, BotError>
	where
		T: AsRef<str>
	{
		let text = text.as_ref();

		self.api
			.send(
				chat.text(text)
			)
			.await
	}


	async fn validate_chat(&self, update_kind: &UpdateKind) -> bool {
		if let Some(chat_info) = ChatInfo::from_update(update_kind) {
			let ChatInfo { chat_id, username, title } = chat_info;

			if Message::is_new_chat(self.username, update_kind) {
				log::warn!("I've been added to a new chat: {:?}", chat_info);
				log::info!("Requesting password...");

				let result = self
					.send_message(chat_id, &self.authentication.prompt)
					.await;

				if let Err(error) = result {
					log::warn!("Failed to send message to chat {:?}: {}", chat_info, error);
				}

				return false;
			}

			let trusted = self.trusted_chats
				.borrow()
				.contains(&chat_id);

			if !trusted {
				match Message::get_text(update_kind) {
					Some(password) if password == self.authentication.password.as_ref() => {
						let result = self.db.new_trusted_chat(
							&NewTrustedChat {
								id: DbChatId(chat_id),
								username,
								title,
							}
						);

						if let Err(error) = result {
							log::warn!("Failed to add trusted chat {:?}: {}", chat_info, error);
							return false; // Don't update the cache if the DB update failed.
						}

						self.trusted_chats
							.borrow_mut()
							.insert(chat_id);

						log::info!("Added trusted chat: {:?}", chat_info);

						let result = self
							.send_message(chat_id, &self.authentication.authorized)
							.await;

						if let Err(error) = result {
							log::warn!("Failed to send message to chat {:?}: {}", chat_info, error);
						}
					},

					_ => self.leave_chat(chat_id).await,
				}
			}

			return trusted;
		}

		false
	}
}


impl<'a> Debug for Bot<'a> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"Bot {{{:?}}}",
			self.trusted_chats
		)
	}
}
