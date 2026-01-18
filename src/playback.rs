// ══════════════════════════════════════════════════════════════════════════════
// PLAYBACK MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Handles audio playback from cassette files. Extracts tracks from memory and
// plays them using rodio. Supports random track selection for testing.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Cursor};
use rand::Rng;
use rodio::{Decoder, OutputStreamBuilder, Sink};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use crate::io::{open_file, find_iend, format_duration};
use crate::logger::{log, LogLevel};

/// Helper function to load cassette TOC and track data
fn load_cassette_toc(path: &str) -> Option<(File, Vec<(String, u64)>, Vec<u64>)> {
	let mut file = match open_file(path) {
		Ok(f) => f,
		Err(e) => { log(LogLevel::Error, &e); return None; }
	};

	// Find TOC position (after IEND)
	let toc_pos = match find_iend(&mut file) {
		Some(pos) => pos,
		None => { log(LogLevel::Error, "This cassette appears to be blank. No IEND chunk found."); return None; }
	};

	// Read TOC
	file.seek(SeekFrom::Start(toc_pos)).unwrap();

	let mut count_buf = [0u8; 4];
	file.read_exact(&mut count_buf).unwrap();
	let track_count = u32::from_le_bytes(count_buf) as usize;

	if track_count == 0 {
		log(LogLevel::Error, "This cassette is blank. No tracks found.");
		return None;
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

	Some((file, entries, offsets))
}

/// Helper function to play a single track
fn play_track(file: &mut File, entries: &[(String, u64)], offsets: &[u64], track_idx: usize, show_selection: bool) -> bool {
	let (ref name, size) = entries[track_idx];
	let track_offset = offsets[track_idx];

	if show_selection {
		log(LogLevel::Info, &format!("Selected track {} of {}: {}", track_idx + 1, entries.len(), name));
	}

	if show_selection {
		log(LogLevel::Info, &format!("Selected track {} of {}: {}", track_idx + 1, entries.len(), name));
	}

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

	// Play audio
	let stream_handle = match OutputStreamBuilder::open_default_stream() {
		Ok(s) => s,
		Err(e) => { log(LogLevel::Error, &format!("Cannot access audio output device: {}", e)); return false; }
	};

	let sink = Sink::connect_new(&stream_handle.mixer());

	let cursor = Cursor::new(audio_data);
	let source = match Decoder::new(cursor) {
		Ok(s) => s,
		Err(e) => { log(LogLevel::Error, &format!("This track is damaged and cannot be played: {}", e)); return false; }
	};

	sink.append(source);

	// Block until done
	sink.sleep_until_end();

	true
}

/// Plays a random track from the cassette file.
/// Blocks until the track finishes or Ctrl+C is pressed.
pub fn play_random(path: &str) {
	log(LogLevel::Info, &format!("Loading cassette: {}", path));

	let (mut file, entries, offsets) = match load_cassette_toc(path) {
		Some(data) => data,
		None => return,
	};

	// Pick random track
	let mut rng = rand::rng();
	let track_idx = rng.random_range(0..entries.len());

	log(LogLevel::Info, "Press Ctrl+C to stop.");
	
	if play_track(&mut file, &entries, &offsets, track_idx, true) {
		log(LogLevel::Success, "Playback finished.");
	}
}

/// Plays all tracks sequentially from the cassette file.
/// Blocks until all tracks finish or Ctrl+C is pressed.
pub fn play_all(path: &str) {
	log(LogLevel::Info, &format!("Loading cassette: {}", path));

	let (mut file, entries, offsets) = match load_cassette_toc(path) {
		Some(data) => data,
		None => return,
	};

	log(LogLevel::Info, &format!("Playing all {} track(s) in sequence...", entries.len()));
	log(LogLevel::Info, "Press Ctrl+C to stop.");

	for i in 0..entries.len() {
		log(LogLevel::Info, &format!("\n━━━ Track {} of {} ━━━", i + 1, entries.len()));
		
		if !play_track(&mut file, &entries, &offsets, i, false) {
			break;
		}
		
		// Small pause between tracks
		if i < entries.len() - 1 {
			std::thread::sleep(std::time::Duration::from_millis(500));
		}
	}

	log(LogLevel::Success, "All tracks played. Cassette complete.");
}
