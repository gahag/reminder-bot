#[macro_use] extern crate diesel;

mod bot;
mod config;
mod controller;
mod db;

use std::{fs::File, io::Read};

use anyhow::Context;

use simple_logger::SimpleLogger;

use tokio::signal::unix::SignalKind;

use crate::{
	config::Config,
	bot::Bot,
	db::Db,
};


const CONFIG_FILE: &'static str = "./config.toml";


#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
	SimpleLogger
		::new()
		.with_level(log::LevelFilter::Info)
		.init()?;

	let config = load_config()?;

	let db = Db
		::open(&config.bot.db)
		.with_context(
			|| format!("failed to open database: {}", config.bot.db)
		)?;

	let bot = Bot::new(
		config.bot.key,
		&db,
		&config.bot.username,
		&config.bot.authentication
	)?;

	let mut sigint = signal(SignalKind::interrupt())?;
	let mut sigquit = signal(SignalKind::quit())?;
	let mut sigterm = signal(SignalKind::terminate())?;

	loop {
		tokio::select! {
			_ = sigint.recv() => {
				log::info!("Interrupted!");
				break;
			}

			_ = sigquit.recv() => {
				log::info!("Exited!");
				break;
			}

			_ = sigterm.recv() => {
				log::info!("Terminated!");
				break;
			}

			// The bot future should never finish, and when it does, it should always return an
			// error.
			Err(error) = launch_bot(&db, &bot, &config.commands, &config.messages) => {
				log::error!("Bot future halted: {}", error);
				break;
			}

			// The notificator future should never finish, and when it does, it should always
			// return an error.
			Err(error) = launch_notificator(&db, &bot) => {
				log::error!("Notificator future halted: {}", error);
				break;
			}
		}
	}

	Ok(())
}


fn signal(signal: SignalKind) -> anyhow::Result<tokio::signal::unix::Signal> {
	tokio::signal::unix
		::signal(signal)
		.with_context(
			|| format!("failed to register signal: {:?}", signal)
		)
}


fn load_config() -> anyhow::Result<Config> {
	let mut config_file = File
		::open(CONFIG_FILE)
		.with_context(
			|| format!("failed to open config file: {}", CONFIG_FILE)
		)?;

	let mut data = Vec::new();

	config_file
		.read_to_end(&mut data)
		.with_context(
			|| format!("failed to read config file: {}", CONFIG_FILE)
		)?;

	let config = Config
		::from_toml(&data)
		.with_context(
			|| format!("failed to parse config file: {}", CONFIG_FILE)
		)?;

	Ok(config)
}


// Should loop forever, or return an error.
async fn launch_bot<'a>(
	db: &'a Db,
	bot: &'a Bot<'a>,
	commands: &'a config::Commands,
	messages: &'a config::Messages,
) -> anyhow::Result<()> {
	controller
		::launch_bot(db, bot, commands, messages)
		.await;

	Err(
		anyhow::anyhow!("controller halted")
	)
}


// Should loop forever, or return an error.
async fn launch_notificator<'a>(db: &'a Db, bot: &'a Bot<'a>) -> anyhow::Result<()> {
	controller
		::launch_notificator(db, bot)
		.await;

	Err(
		anyhow::anyhow!("controller halted")
	)
}
