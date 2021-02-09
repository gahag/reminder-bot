pub mod models;
pub mod schema;
pub mod types;

use diesel::{
	BoolExpressionMethods,
	Connection,
	SqliteConnection,
	QueryDsl,
	RunQueryDsl,
	ExpressionMethods
};
pub use diesel::result::Error;

use self::{
	models::{
		reminders::{NewReminder, Reminder},
		trusted_chats::NewTrustedChat,
	},
	schema::{
		reminders::dsl::{
			id as reminder_id,
			chat as reminder_chat,
			due as reminder_due,
			reminders as reminders_db
		},
		trusted_chats::dsl::{
			id as trusted_chat_id,
			trusted_chats as trusted_chats_db,
		},
	},
	types::{ChatId, DueTimestamp}
};


pub struct Db(SqliteConnection);


impl Db {
	pub fn open(path: &str) -> diesel::ConnectionResult<Self> {
		let connection = SqliteConnection::establish(path)?;

		Ok(
			Self(connection)
		)
	}


	pub fn trusted_chat_ids(&self) -> Result<Box<[ChatId]>, Error> {
		trusted_chats_db
			.select(trusted_chat_id)
			.load::<ChatId>(&self.0)
			.map(Vec::into_boxed_slice)
	}


	pub fn new_trusted_chat<'a>(&self, chat: &NewTrustedChat<'a>) -> Result<(), Error> {
		diesel
			::insert_into(trusted_chats_db)
			.values(chat)
			.execute(&self.0)
			.map(
				|_| ()
			)
	}


	pub fn chat_reminders(&self, chat_id: ChatId) -> Result<Box<[Reminder]>, Error> {
		reminders_db
			.filter(
				reminder_chat.eq(chat_id)
			)
			.load::<Reminder>(&self.0)
			.map(Vec::into_boxed_slice)
	}


	pub fn past_due_reminders(&self, now: DueTimestamp) -> Result<Box<[Reminder]>, Error> {
		reminders_db
			.filter(
				reminder_due.lt(now)
			)
			.load::<Reminder>(&self.0)
			.map(Vec::into_boxed_slice)
	}


	pub fn new_reminder<'a>(&self, reminder: &NewReminder<'a>) -> Result<(), Error> {
		diesel
			::insert_into(reminders_db)
			.values(reminder)
			.execute(&self.0)
			.map(
				|_| ()
			)
	}


	pub fn recur_reminder(&self, reminder: &Reminder) -> Result<bool, Error> {
		let id = reminder.id;
		let recurrence = reminder
			.recurrying
			.ok_or(Error::NotFound)?;

		let new_due = recurrence.advance(reminder.due);

		diesel
			::update(
				reminders_db.filter(
					reminder_id.eq(id)
				)
			)
			.set(
				reminder_due.eq(new_due)
			)
			.execute(&self.0)
			.map(
				|rows_affected| rows_affected == 1
			)
	}


	pub fn delete_reminder(&self, id: i32) -> Result<bool, Error> {
		diesel
			::delete(
				reminders_db
					.filter(
						reminder_id.eq(id)
					)
			)
			.execute(&self.0)
			.map(
				|rows_affected| rows_affected == 1
			)
	}


	pub fn delete_chat_reminder(&self, id: i32, chat_id: ChatId) -> Result<bool, Error> {
		diesel
			::delete(
				reminders_db.filter(
					reminder_id
						.eq(id)
						.and(
							reminder_chat.eq(chat_id)
						)
				)
			)
			.execute(&self.0)
			.map(
				|rows_affected| rows_affected == 1
			)
	}
}
