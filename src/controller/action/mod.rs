mod parser;

use std::fmt::Write;

use thiserror::Error;

use chrono::NaiveDateTime as DateTime;

pub use parser::ParseError;
use crate::{
	bot::{self, Bot, BotError},
	config,
	db::{
		Db,
		Error as DbError,
		models::reminders::NewReminder,
		types::{
			ChatId,
			DueTimestamp,
			Recurrence,
			RecurrenceUnit,
		},
	},
};


#[derive(Debug, Error)]
pub enum ExecutionError {
	#[error("database error: {0}")]
	Db(DbError),
	#[error("bot error: {0}")]
	Bot(BotError),
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AddReminder {
	due: DateTime,
	recurrence: Option<Recurrence>,
	message: Box<str>,
	chat_id: bot::ChatId,
}


impl AddReminder {
	pub async fn execute<'a>(
		self,
		db: &'a Db,
		bot: &'a Bot<'a>,
		messages: &'a config::Messages,
	) -> Result<(), ExecutionError> {
		let reminder = NewReminder {
			due: DueTimestamp(self.due),
			recurrying: self.recurrence,
			chat: ChatId(self.chat_id),
			message: &self.message,
		};

		db
			.new_reminder(&reminder)
			.map_err(ExecutionError::Db)?;

		let mut message = String::new();

		write!(
			message,
			"{}\n{}",
			messages.added_message(),
			reminder
		)
			.expect("write should not fail on string");

		bot
			.send_message(self.chat_id, message)
			.await
			.map_err(ExecutionError::Bot)?;

		Ok(())
	}
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RemoveReminder {
	reminder_id: i32,
	chat_id: bot::ChatId,
}


impl RemoveReminder {
	pub async fn execute<'a>(
		self,
		db: &'a Db,
		bot: &'a Bot<'a>,
		messages: &'a config::Messages,
	) -> Result<(), ExecutionError> {
		let success = db
			.delete_chat_reminder(self.reminder_id, ChatId(self.chat_id))
			.map_err(ExecutionError::Db)?;

		let message =
			if success {
				messages.removed_message()
			} else {
				messages.not_found_message()
			};

		bot
			.send_message(self.chat_id, message)
			.await
			.map_err(ExecutionError::Bot)?;

		Ok(())
	}
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListReminders{
	chat_id: bot::ChatId,
}


impl ListReminders {
	pub async fn execute<'a>(
		self,
		db: &'a Db,
		bot: &'a Bot<'a>,
		messages: &'a config::Messages,
	) -> Result<(), ExecutionError> {
		let reminders = db
			.chat_reminders(ChatId(self.chat_id))
			.map_err(ExecutionError::Db)?
			.into_vec();

		let mut text;

		if reminders.is_empty() {
			text = String::from(
				messages.empty_message()
			);
		}
		else {
			text = format!("{}\n", messages.list_header_message());

			for reminder in reminders {
				writeln!(text, "{}", reminder)
					.expect("write should not fail on string");
			}
		}

		bot
			.send_message(self.chat_id, text)
			.await
			.map_err(ExecutionError::Bot)?;

		Ok(())
	}
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
	AddReminder(AddReminder),
	RemoveReminder(RemoveReminder),
	ListReminders(ListReminders),
}


impl Action {
	pub fn parse<'a>(
		commands: &'a config::Commands,
		chat_id: bot::ChatId,
		input: &'a str
	) -> Result<Self, ParseError<'a>> {
		parser::parse(commands, chat_id, input)
	}


	pub async fn execute<'a>(
		self,
		db: &'a Db,
		bot: &'a Bot<'a>,
		messages: &'a config::Messages,
	) -> Result<(), ExecutionError> {
		match self {
			Action::AddReminder(action) => action.execute(db, bot, messages).await,
			Action::RemoveReminder(action) => action.execute(db, bot, messages).await,
			Action::ListReminders(action) => action.execute(db, bot, messages).await,
		}
	}
}
