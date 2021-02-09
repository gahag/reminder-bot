use std::{
	convert::TryFrom,
	fmt::{self, Display},
	io
};

use thiserror::Error;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};

use diesel::{backend::Backend, deserialize, serialize::{self, Output}, sql_types::{BigInt, Integer}, types::{FromSql, ToSql}};

use crate::bot;


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(AsExpression, FromSqlRow)]
#[sql_type = "BigInt"]
pub struct DueTimestamp(pub NaiveDateTime);


impl Display for DueTimestamp {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"{}",
			self.0.format("%Y-%m-%d %H:%M")
		)
	}
}


impl<DB: Backend> ToSql<BigInt, DB> for DueTimestamp
where
	i64: ToSql<BigInt, DB>,
{
	fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
	where
		W: io::Write,
	{
		let epoch_seconds = self.0
			.timestamp();

		epoch_seconds.to_sql(out)
	}
}


impl<DB: Backend> FromSql<BigInt, DB> for DueTimestamp
where
	i64: FromSql<BigInt, DB>,
{
	fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
		let epoch_seconds = i64::from_sql(bytes)?;

		Ok(
			DueTimestamp(
				NaiveDateTime::from_timestamp(epoch_seconds, 0)
			)
		)
	}
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum RecurrenceUnit {
	Minutes = 1,
	Hours,
	Days,
	Weeks,
	Months,
	Years,
}


#[derive(Debug, Error)]
#[error("invalid value for RecurrenceUnit: {0}")]
pub struct RecurrenceUnitParseError(u8);


impl TryFrom<u8> for RecurrenceUnit {
	type Error = RecurrenceUnitParseError;

	fn try_from(v: u8) -> Result<Self, Self::Error> {
		match v {
			x if x == RecurrenceUnit::Minutes as u8 => Ok(RecurrenceUnit::Minutes),
			x if x == RecurrenceUnit::Hours   as u8 => Ok(RecurrenceUnit::Hours),
			x if x == RecurrenceUnit::Days    as u8 => Ok(RecurrenceUnit::Days),
			x if x == RecurrenceUnit::Weeks   as u8 => Ok(RecurrenceUnit::Weeks),
			x if x == RecurrenceUnit::Months  as u8 => Ok(RecurrenceUnit::Months),
			x if x == RecurrenceUnit::Years   as u8 => Ok(RecurrenceUnit::Years),
			_ => Err(
				RecurrenceUnitParseError(v)
			),
		}
	}
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(AsExpression, FromSqlRow)]
#[sql_type = "Integer"]
pub struct Recurrence {
	pub ammount: u8,
	pub unit: RecurrenceUnit,
}


impl Display for Recurrence {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"+{}{}",
			self.ammount,
			match self.unit {
				RecurrenceUnit::Minutes => 'm',
				RecurrenceUnit::Hours   => 'h',
				RecurrenceUnit::Days    => 'd',
				RecurrenceUnit::Weeks   => 'w',
				RecurrenceUnit::Months  => 'M',
				RecurrenceUnit::Years   => 'y',
			}
		)
	}
}


impl Recurrence {
	pub fn advance(&self, timestamp: DueTimestamp) -> DueTimestamp {
		let timestamp = timestamp.0;

		let new_timestamp = match self.unit {
			RecurrenceUnit::Minutes => timestamp + Duration::minutes(self.ammount as i64),
			RecurrenceUnit::Hours   => timestamp + Duration::hours(self.ammount as i64),
			RecurrenceUnit::Days    => timestamp + Duration::days(self.ammount as i64),
			RecurrenceUnit::Weeks   => timestamp + Duration::weeks(self.ammount as i64),

			RecurrenceUnit::Months => {
				let date = timestamp.date();
				let year = date.year();
				let month = date.month() + self.ammount as u32;
				let day = date.day();

				let overflow = (month / 12) as i32;
				let month = month % 12;

				NaiveDateTime::new(
					NaiveDate::from_ymd(year + overflow, month, day),
					timestamp.time()
				)
			}

			RecurrenceUnit::Years => {
				let date = timestamp.date();
				let year = date.year() + self.ammount as i32;
				let month = date.month();
				let day = date.day();

				NaiveDateTime::new(
					NaiveDate::from_ymd(year, month, day),
					timestamp.time()
				)
			}
		};

		DueTimestamp(new_timestamp)
	}
}


impl<DB: Backend> ToSql<Integer, DB> for Recurrence
where
	i32: ToSql<Integer, DB>,
{
	fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
	where
		W: io::Write,
	{
		let bytes = [ self.ammount, 0, 0, self.unit as u8 ];

		let serialized = i32::from_le_bytes(bytes);

		serialized.to_sql(out)
	}
}


impl<DB: Backend> FromSql<Integer, DB> for Recurrence
where
	i32: FromSql<Integer, DB>,
{
	fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
		let serialized = i32::from_sql(bytes)?;

		let [ ammount, _, _, unit ] = serialized.to_le_bytes();

		let unit = RecurrenceUnit::try_from(unit)?;

		Ok(
			Recurrence { ammount, unit }
		)
	}
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[derive(AsExpression, FromSqlRow)]
#[sql_type = "BigInt"]
pub struct ChatId(pub bot::ChatId);


impl From<i64> for ChatId {
	fn from(value: i64) -> Self {
		Self(value.into())
	}
}


impl<'a> Into<i64> for &'a ChatId {
	fn into(self) -> i64 {
		self.0.into()
	}
}


impl<DB: Backend> ToSql<BigInt, DB> for ChatId
where
	i64: ToSql<BigInt, DB>,
{
	fn to_sql<W>(&self, out: &mut Output<W, DB>) -> serialize::Result
	where
		W: io::Write,
	{
		let id: i64 = self.into();

		id.to_sql(out)
	}
}


impl<DB: Backend> FromSql<BigInt, DB> for ChatId
where
	i64: FromSql<BigInt, DB>,
{
	fn from_sql(bytes: Option<&DB::RawValue>) -> deserialize::Result<Self> {
		let id = i64::from_sql(bytes)?;

		Ok(
			ChatId::from(id)
		)
	}
}
