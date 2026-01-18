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
mod extract;
mod tui;

use clap::{Parser, Subcommand};
use record::record;
use inspect::inspect;
use playback::{play_random, play_all};
use extract::extract;
use tui::run_tui;
use crate::logger::{log, LogLevel};
use colored::*;

/// Digital cassette tapes disguised as PNG images
#[derive(Parser)]
#[command(name = "rewind")]
#[command(version = "0.3.0")]
#[command(about = "Embed and play audio from PNG images", long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Inject audio files into a PNG image to create a cassette
	Record {
		/// Path to the PNG image (cover art)
		image: String,

		/// Audio files to embed (FLAC/MP3/OGG/WAV)
		#[arg(required = true)]
		audio_files: Vec<String>,

		/// Output cassette file path
		#[arg(short, long)]
		output: String,
	},

	/// Inspect a cassette file and verify its integrity
	Inspect {
		/// Path to the cassette file
		cassette: String,
	},

	/// Play a track from the cassette
	Play {
		/// Path to the cassette file
		cassette: String,

		/// Track number to play (if not specified, plays random track)
		#[arg(short, long)]
		track: Option<usize>,

		/// Play all tracks in sequence
		#[arg(short, long)]
		all: bool,
	},

	/// Extract a track from the cassette
	Extract {
		/// Path to the cassette file
		cassette: String,

		/// Track number to extract (1-based index)
		track: usize,

		/// Output file path
		#[arg(short, long)]
		output: String,
	},

	/// Open the interactive TUI player
	Tui {
		/// Path to the cassette file
		cassette: String,
	},
}

fn main() {
	log(LogLevel::Info, &format!("Welcome to {}! {}", "Rewind.png".cyan(), "[●▪▪●]".bold()));

	let cli = Cli::parse();

	match cli.command {
		Commands::Record { image, audio_files, output } => {
			let audio_refs: Vec<&str> = audio_files.iter().map(|s| s.as_str()).collect();
			record(&image, &audio_refs, &output);
		}

		Commands::Inspect { cassette } => {
			inspect(&cassette);
		}

		Commands::Play { cassette, track, all } => {
			if all {
				play_all(&cassette);
			} else if let Some(_track_num) = track {
				log(LogLevel::Warning, "Track selection not yet implemented. Playing random track.");
				play_random(&cassette);
			} else {
				play_random(&cassette);
			}
		}

		Commands::Extract { cassette, track, output } => {
			extract(&cassette, track, &output);
		}

		Commands::Tui { cassette } => {
			if let Err(e) = run_tui(&cassette) {
				log(LogLevel::Error, &e);
			}
		}
	}
}