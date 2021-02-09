use serde::Deserialize;


type Str = Box<str>;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Deserialize)]
pub struct Authentication {
	pub prompt: Str,
	pub password: Str,
	pub authorized: Str,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Deserialize)]
pub struct Bot {
	pub db: Str,
	pub key: Str,
	pub username: Str,
	pub authentication: Authentication,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Deserialize)]
pub struct Commands {
	pub remove_command: Box<Str>,
	pub list_command: Box<Str>,
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Deserialize)]
pub struct Messages {
	pub added_messages: Box<[Str]>,
	pub removed_messages: Box<[Str]>,
	pub not_found_messages: Box<[Str]>,
	pub empty_messages: Box<[Str]>,
	pub list_header_messages: Box<[Str]>,
	pub misunderstanding_messages: Box<[Str]>,
}


macro_rules! pick_message_fn {
	($messages:ident, $func_name:ident) => {
		pub fn $func_name(&self) -> &str {
			&self.$messages[
				fastrand::usize(.. self.$messages.len())
			]
		}
	};
}

impl Messages {
	pick_message_fn!(added_messages, added_message);
	pick_message_fn!(removed_messages, removed_message);
	pick_message_fn!(not_found_messages, not_found_message);
	pick_message_fn!(empty_messages, empty_message);
	pick_message_fn!(list_header_messages, list_header_message);
	pick_message_fn!(misunderstanding_messages, misunderstanding_message);
}


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive(Deserialize)]
pub struct Config {
	pub bot: Bot,
	pub commands: Commands,
	pub messages: Messages,
}


impl Config {
	pub fn from_toml(toml: &[u8]) -> Result<Self, toml::de::Error> {
		toml::from_slice(toml)
	}
}
