mod action;

use futures::StreamExt;

use action::Action;
use crate::{
	bot::{Bot, Message},
	config,
	db::{
		Db,
		Error as DbError,
		models::reminders::Reminder,
		types::DueTimestamp
	},
};


const NOTIFICATOR_INTERVAL: u64 = 5 * 60;


pub async fn launch_bot<'a>(
	db: &'a Db,
	bot: &'a Bot<'a>,
	commands: &'a config::Commands,
	messages: &'a config::Messages,
) {
	log::info!("Bot online!");

	let message_stream = bot.stream();
	futures::pin_mut!(message_stream);

	while let Some(message) = message_stream.next().await {
		match message {
			Message::Text { chat_id, text, .. } => {
				match Action::parse(commands, chat_id, &text) {
					Ok(action) => {
						let result = action.execute(db, bot, messages).await;

						if let Err(error) = result {
							log::warn!("Error when executing action: {}", error);
						}
					}

					Err(_) => {
						let message = messages.misunderstanding_message();

						let result = bot
							.send_message(chat_id, message)
							.await;

						if let Err(error) = result {
							log::warn!("Error when sending message: {}", error);
						}
					}
				}
			}
		}
	};
}


pub async fn launch_notificator<'a>(db: &'a Db, bot: &'a Bot<'a>) {
	log::info!("Notificator online!");

	let mut interval = tokio::time::interval(
		std::time::Duration::from_secs(NOTIFICATOR_INTERVAL)
	);

	loop {
		interval
			.tick()
			.await;

		if let Err(errors) = run_notifications(db, bot).await {
			for error in errors.into_vec() { // Box has no owned iterator.
				log::error!("Failed to run reminder: {}", error);
			}
		}
	}
}


async fn run_notifications<'a>(
	db: &'a Db,
	bot: &'a Bot<'a>
) -> Result<(), Box<[action::ExecutionError]>> {
	log::info!("Running reminders...");

	let now = DueTimestamp(
		chrono::Local
			::now()
			.naive_local()
	);

	let reminders = db
		.past_due_reminders(now)
		.map_err(
			|error| vec![action::ExecutionError::Db(error)]
				.into_boxed_slice()
		)?;

	let mut errors = Vec::new();

	for reminder in reminders.into_vec() {
		log::info!("Sending reminder to {:?}: {}", reminder.chat.0, reminder.message);

		let result = bot.send_message(reminder.chat.0, &reminder.message).await;

		if let Err(error) = result {
			errors.push(
				action::ExecutionError::Bot(error)
			);

			continue;
		}

		let result = reminder_done(db, &reminder);

		if let Err(error) = result {
			errors.push(
				action::ExecutionError::Db(error)
			);
		}
	}

	if errors.is_empty() {
		Ok(())
	}
	else {
		Err(
			errors.into_boxed_slice()
		)
	}
}


fn reminder_done(db: &Db, reminder: &Reminder) -> Result<(), DbError> {
	if reminder.is_recurrying() { // Reminder is recurrying, update.
		let success = db .recur_reminder(&reminder)?;

		if !success {
			log::warn!("Failed to update reminder {:?}.", reminder.id);
		}
	}
	else { // Delete the reminder.
		let success = db.delete_reminder(reminder.id)?;

		if !success {
			log::warn!("Failed to delete reminder {:?}: no such reminder.", reminder.id);
		}
	}

	Ok(())
}
