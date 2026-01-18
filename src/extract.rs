// ══════════════════════════════════════════════════════════════════════════════
// EXTRACT MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Handles extraction of individual tracks from cassette files. Reads the TOC,
// locates the requested track, and writes it to a separate audio file.

use std::io::{Read, Seek, SeekFrom, Write};
use crate::io::{open_file, create_file, find_iend};
use crate::logger::{log, LogLevel};

/// Extracts a specific track from the cassette file to a separate audio file.
pub fn extract(cassette_path: &str, track_number: usize, output_path: &str) {
	log(LogLevel::Info, &format!("Ejecting track {} from cassette: {}", track_number, cassette_path));

	let mut file = match open_file(cassette_path) {
		Ok(f) => f,
		Err(e) => { log(LogLevel::Error, &e); return; }
	};

	// Find TOC position (after IEND)
	let toc_pos = match find_iend(&mut file) {
		Some(pos) => pos,
		None => { log(LogLevel::Error, "This cassette appears to be blank. No IEND chunk found."); return; }
	};

	// Read TOC
	file.seek(SeekFrom::Start(toc_pos)).unwrap();

	let mut count_buf = [0u8; 4];
	file.read_exact(&mut count_buf).unwrap();
	let track_count = u32::from_le_bytes(count_buf) as usize;

	if track_count == 0 {
		log(LogLevel::Error, "This cassette is blank. No tracks found.");
		return;
	}

	if track_number == 0 || track_number > track_count {
		log(LogLevel::Error, &format!("Track {} not found. This cassette has {} track(s).", track_number, track_count));
		return;
	}

	// Parse TOC entries
	let mut entries: Vec<(String, u64)> = Vec::new();
	for _ in 0..track_count {
		let mut len_buf = [0u8; 4];
		file.read_exact(&mut len_buf).unwrap();
		let name_len = u32::from_le_bytes(len_buf) as usize;

		let mut name_buf = vec![0u8; name_len];
		file.read_exact(&mut name_buf).unwrap();
		let name = String::from_utf8_lossy(&name_buf).to_string();

		let mut size_buf = [0u8; 8];
		file.read_exact(&mut size_buf).unwrap();
		let size = u64::from_le_bytes(size_buf);

		entries.push((name, size));
	}

	// Calculate track offset
	let audio_start = file.stream_position().unwrap();
	let mut track_offset = audio_start;
	for i in 0..(track_number - 1) {
		track_offset += entries[i].1;
	}

	let (ref name, size) = entries[track_number - 1];

	log(LogLevel::Info, &format!("Located: {} ({} bytes)", name, size));

	// Read track data
	file.seek(SeekFrom::Start(track_offset)).unwrap();
	let mut audio_data = vec![0u8; size as usize];
	file.read_exact(&mut audio_data).unwrap();

	// Write to output file
	let mut output = match create_file(output_path) {
		Ok(f) => f,
		Err(e) => { log(LogLevel::Error, &e); return; }
	};

	if let Err(e) = output.write_all(&audio_data) {
		log(LogLevel::Error, &format!("Failed to save track to disk: {}", e));
		return;
	}

	log(LogLevel::Success, &format!("🎵 Track ejected successfully → {}", output_path));
}
