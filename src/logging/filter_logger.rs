/*!
A simple stdout logger that allows filtering entries.
*/

use std::fmt::{self, Debug};

use log::{Level, Log, Metadata, Record};
pub use log::SetLoggerError;


pub trait Filter: Send + Sync {
	fn is_enabled(&self, record: &Record) -> bool;
}


impl<F> Filter for F
where
	F: Fn(&Record) -> bool + Send + Sync,
{
	fn is_enabled(&self, record: &Record) -> bool {
		self(record)
	}
}


/// A logger that allows filtering entries.
///
/// You should have only a single instance of this in your program.
///
/// ```
/// # use filter_logger::FilterLogger;
/// # use log::Record;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut logger = FilterLogger::new(log::Level::Info);
/// logger.add_filter(
///     |record: &Record| record.args().as_str().map(|a| a.contains("information")) == Some(true)
/// );
/// logger.install()?;
///
/// log::info!("Here is some information.");
/// log::warn!("Here is a warning.");
///
/// # Ok(())
/// # }
/// ```
pub struct Logger {
	level: Level,
	filters: Vec<Box<dyn Filter>>,
}


impl Logger {
	/// Create a logger with a maximum log level.
	pub fn new(level: Level) -> Self {
		Self {
			level,
			filters: Vec::new(),
		}
	}

	/// Initializes the global logger with a the FilterLogger instance.
	/// Returns the installed FilterLogger instance.
	pub fn install(self) -> Result<&'static Self, SetLoggerError> {
		let logger = Box::leak(
			Box::new(self)
		);

		log::set_logger(logger)?;

		log::set_max_level(
			logger.level.to_level_filter()
		);

		Ok(logger)
	}


	/// Add a filter to the logger. All messages will be filtered using such filter.
	pub fn add_filter<F>(&mut self, filter: F)
	where
		F: Filter + Send + Sync + 'static,
	{
		self.filters.push(
			Box::new(filter)
		)
	}


	/// Log a record directly.
	pub fn emit(record: &Record) {
		let target =
			if record.target().is_empty() {
				record
					.module_path()
					.unwrap_or("?")
			} else {
				record.target()
			};

		eprintln!(
			"[{}] {:<5} | {}",
			target,
			record.level(),
			record.args()
		);
	}
}


impl Debug for Logger {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f
			.debug_struct("Logger")
			.field("level", &self.level)
			.field("#filters", &self.filters.len())
			.finish()
	}
}


impl Log for Logger {
	fn enabled(&self, metadata: &Metadata) -> bool {
		metadata.level() <= self.level
	}


	fn log(&self, record: &Record) {
		let enabled = self.enabled(record.metadata())
		           && self.filters.iter().all(|f| f.is_enabled(record));

		if enabled {
			Self::emit(record);
		}
	}


	fn flush(&self) { }
}
