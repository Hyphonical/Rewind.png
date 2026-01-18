// ══════════════════════════════════════════════════════════════════════════════
// LOGGER MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Provides colored, timestamped console logging with different severity levels.
// Used throughout the application to provide clear user feedback during operations.

use colored::*;
use chrono::Local;

#[allow(dead_code)]
pub enum LogLevel {
	Info,
	Success,
	Warning,
	Error,
}

pub fn log(level: LogLevel, message: &str) {
	let timestamp = Local::now().format("%H:%M:%S").to_string();
	let prefix = match level {
		LogLevel::Info => "𝒊 ".blue().bold(),
		LogLevel::Success => "✔ ".green().bold(),
		LogLevel::Warning => "⚠ ".yellow().bold(),
		LogLevel::Error => "✘ ".red().bold(),
	};

	println!("[{}] {} {}", timestamp.dimmed(), prefix, message);
}
