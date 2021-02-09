use std::fmt::{self, Display};

use super::schema::trusted_chats;
pub use super::types::*;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Queryable)]
pub struct TrustedChat {
	pub id: ChatId,
	pub username: Option<String>,
	pub title: Option<String>,
}


impl Display for TrustedChat {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", self.id.0)?;

		if let Some(username) = &self.username {
			write!(f, " {}", username)?;
		}

		if let Some(title) = &self.title {
			write!(f, " {}", title)?;
		}

		Ok(())
	}
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(Insertable)]
#[table_name = "trusted_chats"]
pub struct NewTrustedChat<'a> {
	pub id: ChatId,
	pub username: Option<&'a str>,
	pub title: Option<&'a str>,
}


impl<'a> Display for NewTrustedChat<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self.id.0)?;

		if let Some(username) = self.username {
			write!(f, " {}", username)?;
		}

		if let Some(title) = self.title {
			write!(f, " {}", title)?;
		}

		Ok(())
	}
}
