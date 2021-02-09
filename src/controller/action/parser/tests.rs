use super::*;


// TODO: add more tests

#[test]
fn test_add() {
	let chat_id = 0.into();

	let parse = |input| super
		::parse(chat_id, input)
		.expect("parse failed");

	let date = |str| Date
		::parse_from_str(str, "%Y-%m-%d")
		.expect("invalid date")
		.and_hms(0, 0, 0);

	let datetime = |str| DateTime
		::parse_from_str(str, "%Y-%m-%d %H:%M")
		.expect("invalid datetime");

	assert_eq!(
		parse("2020-02-03 hey"),
		Action::AddReminder {
			due: date("2020-02-03"),
			recurrence: None,
			message: "hey".into(),
			chat_id
		}
	);

	assert_eq!(
		parse("2020-03-02 hey ho"),
		Action::AddReminder {
			due: date("2020-03-02"),
			recurrence: None,
			message: "hey ho".into(),
			chat_id
		}
	);

	assert_eq!(
		parse("2020-02-03 00:00 hey"),
		Action::AddReminder {
			due: datetime("2020-02-03 00:00"),
			recurrence: None,
			message: "hey".into(),
			chat_id
		}
	);

	assert_eq!(
		parse("2020-02-03 23:59 hey"),
		Action::AddReminder {
			due: datetime("2020-02-03 23:59"),
			recurrence: None,
			message: "hey".into(),
			chat_id
		}
	);
}


#[test]
fn test_list() {
	let chat_id = 0.into();

	let parse = |input| super
		::parse(chat_id, input)
		.expect("parse failed");

	assert_eq!(
		parse("chora"),
		Action::ListReminders(chat_id),
	);

	assert_eq!(
		parse("   chora   "),
		Action::ListReminders(chat_id),
	);
}


#[test]
fn test_remove() {
	let chat_id = 0.into();

	let parse = |input| super
		::parse(chat_id, input)
		.expect("parse failed");

	assert_eq!(
		parse("cancela 1"),
		Action::RemoveReminder(chat_id, 1),
	);

	assert_eq!(
		parse("cancela 2147483647"),
		Action::RemoveReminder(chat_id, 2147483647),
	);

	assert_eq!(
		parse("   cancela    2   "),
		Action::RemoveReminder(chat_id, 2),
	);
}

// TODO: negative tests
