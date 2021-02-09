use thiserror::Error;

use telegram_bot::{
	ChatId,
	Message as Msg,
	MessageChat,
	MessageKind,
	UpdateKind,
	User,
};


#[derive(Debug, Clone, PartialEq)]
#[derive(Error)]
pub enum UpdateError {
	#[error("unrelated update: {0:?}")]
	Unrelated(UpdateKind),
	#[error("unsupported update: {0:?}")]
	Unsupported(MessageChat),
	#[error("unrelated message: {0}")]
	UnrelatedMessage(Box<str>),
}


impl UpdateError {
	pub fn log(&self) {
		log::warn!("{}", self);
	}


	pub fn is_unrelated(&self) -> bool {
		matches!(
			self,
			Self::Unrelated(_) | Self::UnrelatedMessage(_)
		)
	}
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Message {
	Text {
		nickname: Box<str>,
		username: Option<Box<str>>,
		chat_id: ChatId,
		text: Box<str>,
	}
}


impl Message {
	pub fn get_text(update: &UpdateKind) -> Option<&str> {
		let message = match update {
		  UpdateKind::Message(message) => Some(&message.kind),
			UpdateKind::ChannelPost(post) => Some(&post.kind),
			_ => None,
		};

		match message? {
			MessageKind::Text { data, .. } => Some(
				data.as_ref()
			),
			_ => None,
		}
	}


	pub fn is_new_chat(username: &str, update: &UpdateKind) -> bool {
		let message = match update {
		  UpdateKind::Message(message) => &message.kind,
			UpdateKind::ChannelPost(post) => &post.kind,
			_ => return false,
		};

		match message {
			MessageKind::NewChatMembers { data } => {
				data
					.iter()
					.find(
						|user| user.username.as_deref() == Some(username),
					)
					.is_some()
			},
			MessageKind::GroupChatCreated => true,
			MessageKind::SupergroupChatCreated => true,
			MessageKind::ChannelChatCreated => true,
			_ => false,
		}
	}


	pub fn from_update(bot_username: &str, update: UpdateKind) -> Result<Self, UpdateError> {
		match update {
			UpdateKind::Message(
				Msg {
					kind: MessageKind::Text {
						data: text,
						..
					},
					from: User {
						first_name: nickname,
						username,
						..
					},
					chat,
					..
				}
			) => {
				let text = Self::parse_text(bot_username, text, &chat)?;

				Ok(
					Self::Text {
						nickname: nickname.into(),
						username: username.map(Into::into),
						chat_id: chat.id(),
						text
					}
				)
			},

			update => Err(
				UpdateError::Unrelated(update)
			),
		}
	}


	pub fn log(&self) {
		match self {
			Self::Text { nickname, username, chat_id, text } => {
				log::info!(
					"Message from {} ({}) in {:?}: {}",
					nickname,
					username
						.as_deref()
						.unwrap_or("?"),
					chat_id,
					text
				);
			}
		}
	}


	fn parse_text<S>(
		username: &str,
		text: S,
		chat: &MessageChat
	) -> Result<Box<str>, UpdateError>
	where
		S: AsRef<str>,
	{
		// It's kinda bad to allocate everytime we parse a message...
		let prefix = format!("@{} ", username);

		let text = text
			.as_ref()
			.trim();

		match chat {
			MessageChat::Private(_) => Ok(
				text.into()
			),
			MessageChat::Group(_) | MessageChat::Supergroup(_) => {
				text
					.strip_prefix(&prefix)
					.ok_or_else(
						|| UpdateError::UnrelatedMessage(
							text.into()
						)
					)
					.map(
						|text| text
							.trim_start()
							.into()
					)
			},
			_ => Err(
				UpdateError::Unsupported(chat.clone())
			)
		}
	}
}
