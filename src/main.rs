// ╔══════════════════════════════════════════════════════════════════════════════╗
// ║                              REWIND.PNG                                      ║
// ║                     Digital Cassette Tape System                             ║
// ╚══════════════════════════════════════════════════════════════════════════════╝
//
// 🎯 PROJECT GOAL
// ---------------
// Rewind.png is an experimental digital media container that treats PNG images
// as "cassette shells." The visible image is the box art, and high-quality audio
// (FLAC/MP3/OGG/WAV) is embedded in the file structure without corrupting the image.
//
// The goal is to bring back the *ritual* of physical media—where music is attached
// to a specific, beautiful visual object—while maintaining modern lossless quality.
//
// 📦 HOW IT WORKS (Polyglot File Structure)
// -----------------------------------------
// Standard image viewers see:
//   [PNG Header] [Image Data] [IEND Chunk] → Stop reading here
//
// Rewind sees:
//   [PNG Header] [Image Data] [IEND Chunk] [Audio Data] [CRC32]
//
// The appended data after IEND is invisible to normal viewers but readable by
// this tool. A CRC32 checksum ensures integrity (detects compression/tampering).
//
// 🛡️ SECURITY MODEL
// -----------------
// - **Whitelist-Only Audio Formats**: Only FLAC, MP3, OGG, WAV are allowed.
// - **CRC32 Integrity Check**: The cassette is "sealed" with a checksum.
//   If the image is re-compressed (e.g., by social media), playback is blocked
//   with a clear error: "This cassette has been damaged.". Unironically, this could
//   serve as a piracy deterrent, as sharing the images would break playback.
//
// 🎨 DESIGN PHILOSOPHY
// --------------------
// - **One File, One Object**: The PNG *is* the cassette. No sidecar files.
// - **Fragile by Design**: Like vinyl records, if you "scratch" the file
//   (transcode/edit it), the music is destroyed. This encourages careful curation.
// - **Retro Aesthetics**: Designed to work with vintage VHS/cassette cover art.
//
// 🔮 FUTURE ROADMAP
// -----------------
// - [ ] Web-based cassette player
//
// 📜 LICENSE: MIT
// 💾 Author: Hyphonical
// 🌐 GitHub: https://github.com/Hyphonical/Rewind.png
//
// ══════════════════════════════════════════════════════════════════════════════

mod logger;
mod constants;
mod io;
mod record;
mod inspect;
mod playback;

use record::record;
use inspect::inspect;
use playback::play_random;

use crate::logger::log;
use crate::logger::LogLevel;

/// Entry point for demonstration purposes.
fn main() {
	let audio_files = vec!["Track_1.flac", "Track_2.flac", "Track_3.flac", "Track_4.flac", "Track_5.flac", "Track_6.flac"];
	record("T-120.png", &audio_files, "Output.png");
	log(LogLevel::Info, &format!("--------------------------------------------"));
	inspect("Output.png");
	log(LogLevel::Info, &format!("--------------------------------------------"));
	play_random("Output.png");
}