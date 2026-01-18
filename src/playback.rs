// ══════════════════════════════════════════════════════════════════════════════
// PLAYBACK MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Handles audio playback from cassette files. Extracts tracks from memory and
// plays them using rodio. Supports random track selection for testing.

use std::io::{Read, Seek, SeekFrom, Cursor};
use rand::Rng;
use rodio::{Decoder, OutputStreamBuilder, Sink};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use crate::io::{open_file, find_iend, format_duration};
use crate::logger::{log, LogLevel};

/// Plays a random track from the cassette file.
/// Blocks until the track finishes or Ctrl+C is pressed.
pub fn play_random(path: &str) {
	log(LogLevel::Info, &format!("Loading cassette: {}", path));

	let mut file = match open_file(path) {
		Ok(f) => f,
		Err(e) => { log(LogLevel::Error, &e); return; }
	};

	// Find TOC position (after IEND)
	let toc_pos = match find_iend(&mut file) {
		Some(pos) => pos,
		None => { log(LogLevel::Error, "No IEND chunk found. Is this a valid cassette?"); return; }
	};

	// Read TOC
	file.seek(SeekFrom::Start(toc_pos)).unwrap();

	let mut count_buf = [0u8; 4];
	file.read_exact(&mut count_buf).unwrap();
	let track_count = u32::from_le_bytes(count_buf) as usize;

	if track_count == 0 {
		log(LogLevel::Error, "No tracks found in cassette.");
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

	// Calculate track offsets
	let audio_start = file.stream_position().unwrap();
	let mut offsets: Vec<u64> = Vec::new();
	let mut offset = audio_start;
	for (_, size) in &entries {
		offsets.push(offset);
		offset += size;
	}

	// Pick random track
	let mut rng = rand::rng();
	let track_idx = rng.random_range(0..track_count);
	let (ref name, size) = entries[track_idx];
	let track_offset = offsets[track_idx];

	log(LogLevel::Info, &format!("Selected track {} of {}: {}", track_idx + 1, track_count, name));

	// Read track into memory
	file.seek(SeekFrom::Start(track_offset)).unwrap();
	let mut audio_data = vec![0u8; size as usize];
	file.read_exact(&mut audio_data).unwrap();

	// Get metadata for display
	let (artist, title, duration_secs) = match Probe::new(Cursor::new(&audio_data)).guess_file_type() {
		Ok(probe) => match probe.read() {
			Ok(tagged) => {
				let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
				let artist = tag.and_then(|t| t.artist()).map(|s| s.to_string()).unwrap_or_else(|| "Unknown".into());
				let title = tag.and_then(|t| t.title()).map(|s| s.to_string()).unwrap_or_else(|| name.clone());
				let duration = tagged.properties().duration().as_secs();
				(artist, title, duration)
			},
			Err(_) => ("Unknown".into(), name.clone(), 0)
		},
		Err(_) => ("Unknown".into(), name.clone(), 0)
	};

	log(LogLevel::Success, &format!("▶ Now Playing: {} - {} [{}]", artist, title, format_duration(duration_secs)));
	log(LogLevel::Info, "Press Ctrl+C to stop.");

	// Play audio
	let stream_handle = match OutputStreamBuilder::open_default_stream() {
		Ok(s) => s,
		Err(e) => { log(LogLevel::Error, &format!("Failed to open audio output: {}", e)); return; }
	};

	let sink = Sink::connect_new(&stream_handle.mixer());

	let cursor = Cursor::new(audio_data);
	let source = match Decoder::new(cursor) {
		Ok(s) => s,
		Err(e) => { log(LogLevel::Error, &format!("Failed to decode audio: {}", e)); return; }
	};

	sink.append(source);

	// Block until done (Ctrl+C will terminate the process)
	sink.sleep_until_end();

	log(LogLevel::Success, "Playback finished.");
}
