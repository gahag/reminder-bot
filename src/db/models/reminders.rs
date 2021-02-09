use std::fmt::{self, Display};

use super::schema::reminders;
pub use super::types::*;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Queryable)]
pub struct Reminder {
	pub id: i32,
	pub due: DueTimestamp,
	pub recurrying: Option<Recurrence>,
	pub chat: ChatId,
	pub message: String,
}


impl Reminder {
	pub fn is_recurrying(&self) -> bool {
		self.recurrying.is_some()
	}
}


impl Display for Reminder {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if let Some(recurrence) = self.recurrying {
			write!(
				f,
				"({}) {} {}: {}",
				self.id,
				self.due,
				recurrence,
				self.message
			)
		}
		else {
			write!(
				f,
				"({}) {}: {}",
				self.id,
				self.due,
				self.message
			)
		}
	}
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Insertable)]
#[table_name = "reminders"]
pub struct NewReminder<'a> {
	pub due: DueTimestamp,
	pub recurrying: Option<Recurrence>,
	pub chat: ChatId,
	pub message: &'a str,
}


impl<'a> Display for NewReminder<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    if let Some(recurrence) = self.recurrying {
			write!(
				f,
				"{} {}: {}",
				self.due,
				recurrence,
				self.message
			)
		}
		else {
			write!(
				f,
				"{}: {}",
				self.due,
				self.message
			)
		}
	}
}
