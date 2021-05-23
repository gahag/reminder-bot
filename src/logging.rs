use std::{
	fmt::Write,
	sync::Mutex,
};

use log::{Level, Record};

use filter_logger::{Filter, FilterLogger};



#[derive(Debug, Copy, Clone)]
struct Metadata {
	level: Level,
	module_path: Option<&'static str>,
	file: Option<&'static str>,
}


impl Metadata {
	fn matches(&self, other: &Record) -> bool {
		self.level == other.level()
			&& self.module_path == other.module_path_static()
			&& self.file == other.file_static()
	}

	fn update(&mut self, from: &Record) {
		self.level = from.level();
		self.module_path = from.module_path_static();
		self.file = from.file_static();
	}
}


impl Default for Metadata {
	fn default() -> Self {
		Self {
			level: Level::max(),
			module_path: None,
			file: None,
		}
	}
}


#[derive(Debug)]
struct PreviousRecord {
	metadata: Metadata,
	message: String,
}


impl PreviousRecord {
	/// Check if the new record matches. If it doesn't, then update self to reflect the new
	/// record, and place the old message in the buffer.
	fn match_update(&mut self, other: &Record, buffer: &mut String) -> Option<Metadata> {
		match other.args().as_str() {
			// msg is static, we can just copy.
			Some(msg) => {
				if self.metadata.matches(other) && self.message == msg {
					None
				} else {
					// Place old message in the buffer.
					std::mem::swap(&mut self.message, buffer);
					self.message.clear();
					self.message.push_str(msg);

					let old_metadata = self.metadata;
					self.metadata.update(other);

					Some(old_metadata)
				}
			},

			// We must use allocated space for the message. This allows us to swap instead of
			// copying if they match.
			None => {
				buffer.clear();
				buffer
					.write_fmt(*other.args())
					.expect("write should never fail for String");

				if self.metadata.matches(other) && self.message == *buffer {
					None
				} else {
					std::mem::swap(&mut self.message, buffer);

					let old_metadata = self.metadata;
					self.metadata.update(other);

					Some(old_metadata)
				}
			}
		}
	}


	fn update(&mut self, record: &Record, buffer: &mut String) {
		match record.args().as_str() {
			// msg is static, we can just copy.
			Some(msg) => {
				// Place old message in the buffer.
				std::mem::swap(&mut self.message, buffer);
				self.message.clear();
				self.message.push_str(msg);

				self.metadata.update(record);
			},

			// We must use allocated space for the message. This allows us to swap instead of
			// copying if they match.
			None => {
				buffer.clear();
				buffer
					.write_fmt(*record.args())
					.expect("write should never fail for String");

				std::mem::swap(&mut self.message, buffer);

				self.metadata.update(record);
			}
		}
	}
}


impl Default for PreviousRecord {
	fn default() -> Self {
		Self {
			metadata: Metadata::default(),
			message: String::new(),
		}
	}
}


#[derive(Debug)]
enum VisitResult {
	Enable,
	Disable,
	Previous {
		record: PreviousRecord,
		repetitions: usize,
	}
}


#[derive(Debug, Default)]
struct RecordData {
	previous_record: PreviousRecord,
	buffer: String,
	repetition_count: usize,
}


impl RecordData {
	fn visit(&mut self, record: &Record, batch_size: usize) -> VisitResult {
		match self.previous_record.match_update(record, &mut self.buffer) {
			None if self.repetition_count >= batch_size => {
				let result = VisitResult::Previous {
					record: PreviousRecord {
						metadata: self.previous_record.metadata,
						message: std::mem::take(&mut self.previous_record.message),
					},
					repetitions: self.repetition_count,
				};

				self.previous_record = PreviousRecord::default();
				self.previous_record.update(record, &mut self.buffer);
				self.repetition_count = 0;

				result
			}

			Some(old_metadata) => {
				if self.repetition_count == 0 {
					VisitResult::Enable
				} else {
					let repetitions = self.repetition_count;
					self.repetition_count = 0;

					VisitResult::Previous {
						record: PreviousRecord {
							metadata: old_metadata,
							message: std::mem::take(&mut self.buffer),
						},
						repetitions,
					}
				}
			}

			None => {
				self.repetition_count += 1;
				VisitResult::Disable
			}
		}
	}
}


struct SpamFilter {
	record_data: Mutex<RecordData>,
	batch_size: usize,
}


impl SpamFilter {
	fn new(batch_size: usize) -> Self
	{
		Self {
			record_data: Mutex::new(RecordData::default()),
			batch_size,
		}
	}
}


impl Filter for SpamFilter {
	fn is_enabled(&self, record: &log::Record) -> bool {
		let result = {
			self.record_data
				.lock()
				.expect("poisoned mutex")
				.visit(record, self.batch_size)
		};

		match result {
			VisitResult::Enable => true,
			VisitResult::Disable => false,
			VisitResult::Previous { record: PreviousRecord { metadata, message }, repetitions } => {
				eprintln!(
					"[{}] {:<5} | {} ({}x)",
					metadata.module_path.unwrap_or("?"),
					metadata.level,
					message,
					repetitions,
				);
				true
			}
		}
	}
}


pub fn setup(batch_size: usize) -> anyhow::Result<()> {
	let mut logger = FilterLogger::new(log::Level::Info);

	logger.add_filter(SpamFilter::new(batch_size));

	logger.install()?;

	Ok(())
}
