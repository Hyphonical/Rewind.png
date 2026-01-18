// ══════════════════════════════════════════════════════════════════════════════
// INSPECT MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Reads and displays metadata from cassette files. Verifies CRC32 integrity,
// parses the table of contents (TOC), and extracts audio metadata (artist, title,
// duration) from embedded tracks using the Lofty library.

use std::io::{Read, Seek, SeekFrom};
use crc32fast::Hasher;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use crate::io::{open_file, hash_only, find_iend, format_duration};
use crate::logger::{log, LogLevel};

pub struct TocEntry {
	pub name: String,
	pub size: u64,
}

/// Inspects the cassette file, verifying integrity and listing audio tracks.
pub fn inspect(path: &str) {
	log(LogLevel::Info, &format!("Inspecting file: {}", path));

	let mut file = match open_file(path) {
		Ok(f) => f,
		Err(e) => { log(LogLevel::Error, &e); return; }
	};

	let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
	if file_len < 4 {
		log(LogLevel::Error, "File too small.");
		return;
	}

	// 1. Verify CRC (single pass)
	let data_len = file_len - 4;
	let mut hasher = Hasher::new();
	hash_only(&mut file, &mut hasher, data_len).unwrap();

	let mut crc_buf = [0u8; 4];
	file.read_exact(&mut crc_buf).unwrap();
	let stored_crc = u32::from_le_bytes(crc_buf);

	if hasher.finalize() != stored_crc {
		log(LogLevel::Error, "Checksum does not match! The file may be corrupted.");
		return;
	}
	log(LogLevel::Success, "Checksum matches. The file is intact.");

	// 2. Find TOC position
	let toc_pos = match find_iend(&mut file) {
		Some(pos) => pos,
		None => { log(LogLevel::Error, "No IEND chunk found."); return; }
	};

	// 3. Read TOC
	file.seek(SeekFrom::Start(toc_pos)).unwrap();

	let mut count_buf = [0u8; 4];
	file.read_exact(&mut count_buf).unwrap();
	let track_count = u32::from_le_bytes(count_buf);

	log(LogLevel::Info, &format!("TOC: {} audio file(s)", track_count));

	let mut toc_entries: Vec<TocEntry> = Vec::new();
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

		toc_entries.push(TocEntry { name, size });
	}

	// 4. Read metadata for each track
	let mut track_offset = file.stream_position().unwrap();

	for (i, entry) in toc_entries.iter().enumerate() {
		file.seek(SeekFrom::Start(track_offset)).unwrap();
		
		// Read the audio chunk into memory for probing
		let mut audio_data = vec![0u8; entry.size as usize];
		file.read_exact(&mut audio_data).unwrap();
		
		let meta = match Probe::new(std::io::Cursor::new(&audio_data)).guess_file_type() {
			Ok(probe) => match probe.read() {
				Ok(tagged) => {
					let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
					let artist = tag.and_then(|t| t.artist()).map(|s| s.to_string()).unwrap_or("-".into());
					let title = tag.and_then(|t| t.title()).map(|s| s.to_string()).unwrap_or("-".into());
					let duration = format_duration(tagged.properties().duration().as_secs());
					format!("🎵 {} - {} [{}]", artist, title, duration)
				},
				Err(e) => format!("(Error reading tags: {})", e)
			},
			Err(e) => format!("(Error probing file: {})", e)
		};

		log(LogLevel::Info, &format!("  [{}] {} ({} bytes) | {}", i + 1, entry.name, entry.size, meta));
		track_offset += entry.size;
	}
}
