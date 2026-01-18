// ══════════════════════════════════════════════════════════════════════════════
// I/O MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Shared I/O utilities used across recording, inspection, and playback modules.
// Handles file operations, data transfer with hashing, PNG structure parsing,
// and audio format validation.

use std::fs::File;
use std::io::{Read, Write, Seek};
use crc32fast::Hasher;
use lofty::probe::Probe;
use crate::constants::{IEND_CHUNK, BUFFER_SIZE};

/// Opens a file with a descriptive error message on failure.
pub fn open_file(path: &str) -> Result<File, String> {
	File::open(path).map_err(|e| format!("Cassette not found in the deck: {} ({})", path, e))
}

/// Creates a file with a descriptive error message on failure.
pub fn create_file(path: &str) -> Result<File, String> {
	File::create(path).map_err(|e| format!("Cannot create output file '{}': {}", path, e))
}

/// Copies all bytes from reader to writer, updating the hasher. Returns bytes written.
pub fn transfer<R: Read, W: Write>(reader: &mut R, writer: &mut W, hasher: &mut Hasher) -> std::io::Result<u64> {
	let mut buffer = [0u8; BUFFER_SIZE];
	let mut total = 0u64;
	loop {
		let n = reader.read(&mut buffer)?;
		if n == 0 { break; }
		writer.write_all(&buffer[..n])?;
		hasher.update(&buffer[..n]);
		total += n as u64;
	}
	Ok(total)
}

/// Hashes all bytes from reader without writing anywhere. Returns bytes read.
pub fn hash_only<R: Read>(reader: &mut R, hasher: &mut Hasher, limit: u64) -> std::io::Result<u64> {
	let mut buffer = [0u8; BUFFER_SIZE];
	let mut total = 0u64;
	while total < limit {
		let to_read = std::cmp::min(BUFFER_SIZE as u64, limit - total) as usize;
		let n = reader.read(&mut buffer[..to_read])?;
		if n == 0 { break; }
		hasher.update(&buffer[..n]);
		total += n as u64;
	}
	Ok(total)
}

/// Scans file for PNG IEND chunk, returns position immediately after it.
pub fn find_iend(file: &mut File) -> Option<u64> {
	file.rewind().ok()?;
	let mut buffer = [0u8; BUFFER_SIZE];
	let mut file_pos = 0u64;

	loop {
		let n = file.read(&mut buffer).ok()?;
		if n == 0 { break; }

		// Scan for IEND signature (need at least 12 bytes to match)
		if n >= IEND_CHUNK.len() {
			for i in 0..=(n - IEND_CHUNK.len()) {
				if buffer[i..i + IEND_CHUNK.len()] == IEND_CHUNK {
					return Some(file_pos + i as u64 + IEND_CHUNK.len() as u64);
				}
			}
		}
		file_pos += n as u64;
	}
	None
}

/// Validates that a file is a supported audio format using Lofty.
pub fn validate_audio(file: &mut File) -> Result<(), String> {
	file.rewind().map_err(|e| e.to_string())?;
	Probe::new(&mut *file)
		.guess_file_type()
		.map_err(|_| "This doesn't sound like music. Unknown format.".to_string())?
		.read()
		.map_err(|_| "This audio file is damaged or corrupted.".to_string())?;
	file.rewind().map_err(|e| e.to_string())?;
	Ok(())
}

/// Formats duration in seconds to "M:SS" string.
pub fn format_duration(secs: u64) -> String {
	format!("{}:{:02}", secs / 60, secs % 60)
}
