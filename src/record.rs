// ══════════════════════════════════════════════════════════════════════════════
// RECORD MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Handles the injection of audio files into PNG images to create cassette files.
// Validates audio formats, builds a table of contents (TOC), appends audio data
// after the PNG IEND chunk, and seals the file with a CRC32 integrity checksum.

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use crc32fast::Hasher;
use crate::io::{open_file, create_file, validate_audio, transfer};
use crate::logger::{log, LogLevel};

/// Injects audio files into the PNG image, producing a cassette file.
pub fn record(image_path: &str, audio_paths: &[&str], output_path: &str) {
	log(LogLevel::Info, &format!("Injecting {} audio file(s) into {}", audio_paths.len(), image_path));

	// 1. Validate and collect audio file info
	let mut audio_files: Vec<(File, String, u64)> = Vec::new();

	for &path in audio_paths {
		let mut file = match open_file(path) {
			Ok(f) => f,
			Err(e) => { log(LogLevel::Error, &e); return; }
		};

		if let Err(e) = validate_audio(&mut file) {
			log(LogLevel::Error, &format!("{}: {}", path, e));
			return;
		}

		let size = file.metadata().map(|m| m.len()).unwrap_or(0);
		audio_files.push((file, path.to_string(), size));
		log(LogLevel::Info, &format!("Validated: {}", path));
	}

	// 2. Open image input and output
	let mut image_in = match open_file(image_path) {
		Ok(f) => f,
		Err(e) => { log(LogLevel::Error, &e); return; }
	};

	let output = match create_file(output_path) {
		Ok(f) => f,
		Err(e) => { log(LogLevel::Error, &e); return; }
	};

	let mut writer = BufWriter::new(output);
	let mut hasher = Hasher::new();

	// 3. Copy image
	if let Err(e) = transfer(&mut BufReader::new(&mut image_in), &mut writer, &mut hasher) {
		log(LogLevel::Error, &format!("Image copy failed: {}", e));
		return;
	}
	log(LogLevel::Info, "Image copied.");

	// 4. Build and write TOC
	let mut toc = Vec::new();
	toc.extend_from_slice(&(audio_files.len() as u32).to_le_bytes());
	for (_, name, size) in &audio_files {
		let name_bytes = name.as_bytes();
		toc.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
		toc.extend_from_slice(name_bytes);
		toc.extend_from_slice(&size.to_le_bytes());
	}
	writer.write_all(&toc).unwrap();
	hasher.update(&toc);
	log(LogLevel::Info, "TOC written.");

	// 5. Append audio data
	for (mut file, name, _) in audio_files {
		if let Err(e) = transfer(&mut BufReader::new(&mut file), &mut writer, &mut hasher) {
			log(LogLevel::Error, &format!("Audio copy failed ({}): {}", name, e));
			return;
		}
		log(LogLevel::Info, &format!("Appended: {}", name));
	}

	// 6. Write CRC
	let crc = hasher.finalize();
	writer.write_all(&crc.to_le_bytes()).unwrap();
	log(LogLevel::Success, &format!("Injection complete. CRC32: {:08X}", crc));
}
