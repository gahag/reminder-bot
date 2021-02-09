#[cfg(test)]
mod tests;

use super::{
	Action,
	AddReminder,
	RemoveReminder,
	ListReminders,
	Recurrence,
	RecurrenceUnit
};

use chrono::{
	NaiveDate as Date,
	NaiveTime as Time,
	NaiveDateTime as DateTime,
};

use combine::{
	EasyParser,
	Parser,
	Stream,
	any,
	attempt,
	choice,
	count_min_max,
	eof,
	error::StreamError,
	from_str,
	many1,
	one_of,
	optional,
	tokens_cmp,
	parser::char::{char, digit, space, spaces}
};

use crate::{bot, config};


pub type ParseError<'a> = combine::easy::ParseError<&'a str>;

pub type Result<'a, T> = std::result::Result<T, ParseError<'a>>;


pub fn parse<'a>(
	commands: &'a config::Commands,
	chat_id: bot::ChatId,
	input: &'a str,
) -> Result<'a, Action> {
	let action = choice!(
		attempt(
			list_command(&commands.list_command, chat_id)
				.map(Action::ListReminders)
		),
		attempt(
			remove_command(&commands.remove_command, chat_id)
				.map(Action::RemoveReminder)
		),
		add_command(chat_id)
			.map(Action::AddReminder)
	);

	let mut parser = (
		action,
		spaces(),
		eof()
	)
		.map(
			|(action, _, _)| action
		);

	parser
		.easy_parse(input)
		.map(
			|(action, _)| action
		)
}


fn case_insensitive(c1: char, c2: char) -> bool {
	c1.eq_ignore_ascii_case(&c2)
}


fn fixed_width_u32<Input>(min: u8, max: u8) -> impl Parser<Input, Output = u32>
where
	Input: Stream<Token = char>
{
	struct Number(u32);

	impl Default for Number {
		fn default() -> Self { Self(0) }
	}

	impl Extend<char> for Number {
		fn extend<I>(&mut self, iter: I)
		where
			I: IntoIterator<Item = char>
		{
			for c in iter {
				let digit = c
					.to_digit(10)
					.expect("tried to convert non digit while parsing");

				self.0 = self.0 * 10 + digit
			}
		}
 }

	let min = min as usize;
	let max = max as usize;

	count_min_max(
		min, max,
		digit()
	)
		.map(
			|n: Number| n.0
		)
}


fn date<Input>() -> impl Parser<Input, Output = Date>
where
	Input: Stream<Token = char>
{
	(
		spaces(),
		fixed_width_u32(4, 4),
		char('-'),
		fixed_width_u32(2, 2),
		char('-'),
		fixed_width_u32(2, 2),
	)
		.and_then(
			|(_, year, _, month, _, day)| Date
				::from_ymd_opt(year as i32, month, day)
				.ok_or(
					<Input::Error as combine::ParseError<_, _, _>>::StreamError::unexpected_format("invalid date")
				)
		)
}


fn time<Input>() -> impl Parser<Input, Output = Time>
where
	Input: Stream<Token = char>
{
	(
		spaces(),
		fixed_width_u32(2, 2),
		char(':'),
		fixed_width_u32(2, 2),
	)
		.and_then(
			|(_, hours, _, minutes)| Time
				::from_hms_opt(hours, minutes, 0)
				.ok_or(
					<Input::Error as combine::ParseError<_, _, _>>::StreamError::unexpected_format("invalid time")
				)
		)
}


fn recurrence<Input>() -> impl Parser<Input, Output = Recurrence>
where
	Input: Stream<Token = char>
{
	let error = |message| <Input::Error as combine::ParseError<_, _, _>>
		::StreamError
		::unexpected_format(message);

	let validate_recurrence = move |max, recurrence: Recurrence| {
		if recurrence.ammount < max {
			Ok(recurrence)
		}
		else {
			Err(
				error("invalid period")
			)
		}
	};

	let units = "hdwmy".chars();

	(
		spaces(),
		char('+'),
		optional(
			fixed_width_u32(1, 2)
		),
		one_of(units)
	)
		.and_then(
			move |(_, _, num, unit)| {
				let ammount = num.unwrap_or(1) as u8;

				match unit {
					'm' => validate_recurrence(90, Recurrence { ammount, unit: RecurrenceUnit::Minutes }),
					'h' => validate_recurrence(24, Recurrence { ammount, unit: RecurrenceUnit::Hours }),
					'd' => validate_recurrence(99, Recurrence { ammount, unit: RecurrenceUnit::Days }),
					'w' => validate_recurrence(10, Recurrence { ammount, unit: RecurrenceUnit::Weeks }),
					'M' => validate_recurrence(64, Recurrence { ammount, unit: RecurrenceUnit::Months }),
					'y' => validate_recurrence(10, Recurrence { ammount, unit: RecurrenceUnit::Years }),

					_ => Err(
						error("failed to parse recurrence")
					)
				}
			}
		)
}


fn list_command<'a, Input: 'a>(
	command: &'a str,
	chat_id: bot::ChatId
) -> impl Parser<Input, Output = ListReminders> + 'a
where
	Input: Stream<Token = char>
{
	(
		spaces(),
		tokens_cmp(command.chars(), case_insensitive),
	)
		.map(
			move |_| ListReminders { chat_id }
		)
}


fn add_command<Input>(chat_id: bot::ChatId) -> impl Parser<Input, Output = AddReminder>
where
	Input: Stream<Token = char>
{
	let opt_time =
		optional(
			attempt(
				space() // Require a space first to separate from the date.
					.with(time())
			)
		)
		.map(
			|time| time.unwrap_or(
				Time::from_hms(0, 0, 0)
			)
		);

	let opt_rec = optional(
		attempt(
			space() // Require a space first to separate from the date/time.
				.with(recurrence())
		)
	);

	(
		spaces(),
		date(),
		opt_time,
		opt_rec,
		space(),
		many1::<String, _, _>(any())
	)
		.map(
			move |(_, date, time, rec, _, mut msg)| {
				msg.truncate(
					msg
						.trim_end()
						.len()
				);

				AddReminder {
					due: DateTime::new(date, time),
					recurrence: rec,
					message: msg.into(),
					chat_id,
				}
			}
		)
}


fn remove_command<'a, Input: 'a>(
	command: &'a str,
	chat_id: bot::ChatId,
) -> impl Parser<Input, Output = RemoveReminder> + 'a
where
	Input: Stream<Token = char>
{
	(
		spaces(),
		tokens_cmp(command.chars(), case_insensitive),
		spaces(),
		from_str(
			many1::<String, _, _>(
				digit()
			)
		),
	)
		.map(
			move |(_, _, _, reminder_id)| RemoveReminder { reminder_id, chat_id }
		)
}
