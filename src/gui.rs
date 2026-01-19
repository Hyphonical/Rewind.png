// ══════════════════════════════════════════════════════════════════════════════
// GUI MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Desktop GUI for Rewind.png cassettes using Dioxus. Provides a visual
// track list, playback controls, and progress display. Minimal prototype

use std::io::{Read, Seek, SeekFrom, Cursor};
use std::sync::{Mutex, OnceLock};

use dioxus::prelude::*;
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;

use crate::io::{open_file, find_iend, format_duration};

// ══════════════════════════════════════════════════════════════════════════════
// DATA STRUCTURES
// ══════════════════════════════════════════════════════════════════════════════

/// Represents a single audio track on the cassette
#[derive(Clone, Debug, PartialEq)]
pub struct Track {
	pub name: String,
	pub size: u64,
	pub offset: u64,
	pub artist: String,
	pub title: String,
	pub duration_secs: u64,
}

/// Player state
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlayerState {
	Stopped,
	Playing,
	Paused,
}

/// Global app data (set before launch)
static APP_DATA: OnceLock<AppData> = OnceLock::new();

#[derive(Clone)]
struct AppData {
	cassette_path: String,
	tracks: Vec<Track>,
}

/// Audio player wrapper - must be kept alive for playback
struct AudioPlayer {
	_stream: OutputStream,
	sink: Sink,
}

impl AudioPlayer {
	fn new() -> Option<Self> {
		let stream = OutputStreamBuilder::open_default_stream().ok()?;
		let sink = Sink::connect_new(&stream.mixer());
		Some(Self {
			_stream: stream,
			sink,
		})
	}
}

/// Global audio player (needs to stay alive)
static AUDIO_PLAYER: OnceLock<Mutex<Option<AudioPlayer>>> = OnceLock::new();

fn get_or_init_player() -> &'static Mutex<Option<AudioPlayer>> {
	AUDIO_PLAYER.get_or_init(|| Mutex::new(AudioPlayer::new()))
}

// ══════════════════════════════════════════════════════════════════════════════
// CASSETTE LOADING
// ══════════════════════════════════════════════════════════════════════════════

/// Load track metadata from a cassette file
fn load_tracks(path: &str) -> Result<Vec<Track>, String> {
	let mut file = open_file(path)?;

	let toc_pos = find_iend(&mut file)
		.ok_or_else(|| "No PNG structure found - is this a valid cassette?".to_string())?;

	file.seek(SeekFrom::Start(toc_pos)).map_err(|e| e.to_string())?;

	let mut count_buf = [0u8; 4];
	file.read_exact(&mut count_buf).map_err(|e| e.to_string())?;
	let track_count = u32::from_le_bytes(count_buf) as usize;

	// Parse TOC entries
	let mut entries: Vec<(String, u64)> = Vec::new();
	for _ in 0..track_count {
		let mut len_buf = [0u8; 4];
		file.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
		let name_len = u32::from_le_bytes(len_buf) as usize;

		let mut name_buf = vec![0u8; name_len];
		file.read_exact(&mut name_buf).map_err(|e| e.to_string())?;
		let name = String::from_utf8_lossy(&name_buf).to_string();

		let mut size_buf = [0u8; 8];
		file.read_exact(&mut size_buf).map_err(|e| e.to_string())?;
		let size = u64::from_le_bytes(size_buf);

		entries.push((name, size));
	}

	// Calculate offsets and load metadata
	let audio_start = file.stream_position().map_err(|e| e.to_string())?;
	let mut tracks = Vec::new();
	let mut offset = audio_start;

	for (name, size) in entries {
		// Read audio data to extract metadata
		file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
		let mut audio_data = vec![0u8; size as usize];
		file.read_exact(&mut audio_data).map_err(|e| e.to_string())?;

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

		tracks.push(Track {
			name,
			size,
			offset,
			artist,
			title,
			duration_secs,
		});

		offset += size;
	}

	Ok(tracks)
}

/// Load raw audio data for a specific track
fn load_track_data(cassette_path: &str, track: &Track) -> Result<Vec<u8>, String> {
	let mut file = open_file(cassette_path)?;
	file.seek(SeekFrom::Start(track.offset)).map_err(|e| e.to_string())?;
	let mut data = vec![0u8; track.size as usize];
	file.read_exact(&mut data).map_err(|e| e.to_string())?;
	Ok(data)
}

// ══════════════════════════════════════════════════════════════════════════════
// GUI ENTRY POINT
// ══════════════════════════════════════════════════════════════════════════════

/// Main entry point for the GUI
pub fn run_gui(cassette_path: &str) -> Result<(), String> {
	let tracks = load_tracks(cassette_path)?;

	// Store app data globally before launch
	APP_DATA.set(AppData {
		cassette_path: cassette_path.to_string(),
		tracks,
	}).map_err(|_| "Failed to initialize app data")?;

	// Initialize audio player
	let _ = get_or_init_player();

	// Launch Dioxus app
	dioxus::LaunchBuilder::desktop()
		.with_cfg(
			dioxus::desktop::Config::new()
				.with_window(
					dioxus::desktop::WindowBuilder::new()
						.with_title("Rewind.png")
						.with_inner_size(dioxus::desktop::LogicalSize::new(500.0, 600.0))
						.with_resizable(true)
				)
		)
		.launch(App);

	Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// GUI COMPONENTS
// ══════════════════════════════════════════════════════════════════════════════

/// Main App component
#[component]
fn App() -> Element {
	// Get app data from global
	let app_data = APP_DATA.get().expect("App data not initialized");
	let tracks = app_data.tracks.clone();
	let cassette_path = app_data.cassette_path.clone();

	// State
	let mut selected_track = use_signal(|| 0usize);
	let mut player_state = use_signal(|| PlayerState::Stopped);
	let mut current_track_idx = use_signal(|| None::<usize>);

	// Get current track info for display
	let current_idx = *current_track_idx.read();
	let now_playing_track = current_idx.map(|idx| tracks[idx].clone());
	let state = *player_state.read();
	let selected = *selected_track.read();

	rsx! {
		style { {CSS} }

		div { class: "app",
			// Header
			div { class: "header",
				"🎵 Rewind.png"
				span { class: "cassette-icon", " [●▪▪●]" }
			}

			// Track list
			div { class: "track-list",
				for (idx, track) in tracks.iter().enumerate() {
					{
						let is_selected = selected == idx;
						let is_playing = current_idx == Some(idx);
						let class_name = if is_selected {
							"track selected"
						} else if is_playing {
							"track playing"
						} else {
							"track"
						};
						let track_for_play = track.clone();
						let path_for_play = cassette_path.clone();

						rsx! {
							div {
								class: "{class_name}",
								onclick: move |_| selected_track.set(idx),
								ondoubleclick: move |_| {
									// Play track on double click
									if let Ok(audio_data) = load_track_data(&path_for_play, &track_for_play) {
										if let Ok(mut guard) = get_or_init_player().lock() {
											// Recreate player to stop previous track
											*guard = AudioPlayer::new();
											if let Some(ref player) = *guard {
												if let Ok(source) = Decoder::new(Cursor::new(audio_data)) {
													player.sink.append(source);
													current_track_idx.set(Some(idx));
													player_state.set(PlayerState::Playing);
												}
											}
										}
									}
								},

								span { class: "track-number", "{idx + 1}." }
								div { class: "track-info",
									div { class: "track-title", "{track.title}" }
									div { class: "track-artist", "{track.artist}" }
								}
								span { class: "track-duration", "{format_duration(track.duration_secs)}" }
							}
						}
					}
				}
			}

			// Now playing
			div { class: "now-playing",
				if let Some(ref track) = now_playing_track {
					div { class: "np-info",
						div { class: "np-title", "{track.title}" }
						div { class: "np-artist", "{track.artist}" }
					}
					div { class: "np-state",
						{match state {
							PlayerState::Playing => "▶ Playing",
							PlayerState::Paused => "⏸ Paused",
							PlayerState::Stopped => "⏹ Stopped",
						}}
					}
				} else {
					div { class: "np-empty", "No track playing" }
				}
			}

			// Controls
			div { class: "controls",
				button {
					onclick: {
						let tracks = tracks.clone();
						let cassette_path = cassette_path.clone();
						move |_| {
							let sel = *selected_track.read();
							if sel < tracks.len() {
								let track = &tracks[sel];
								if let Ok(audio_data) = load_track_data(&cassette_path, track) {
									if let Ok(mut guard) = get_or_init_player().lock() {
										*guard = AudioPlayer::new();
										if let Some(ref player) = *guard {
											if let Ok(source) = Decoder::new(Cursor::new(audio_data)) {
												player.sink.append(source);
												current_track_idx.set(Some(sel));
												player_state.set(PlayerState::Playing);
											}
										}
									}
								}
							}
						}
					},
					"▶ Play"
				}
				button {
					onclick: move |_| {
						if let Ok(guard) = get_or_init_player().lock() {
							if let Some(ref player) = *guard {
								if player.sink.is_paused() {
									player.sink.play();
									player_state.set(PlayerState::Playing);
								} else {
									player.sink.pause();
									player_state.set(PlayerState::Paused);
								}
							}
						}
					},
					"⏸ Pause"
				}
				button {
					onclick: move |_| {
						if let Ok(mut guard) = get_or_init_player().lock() {
							if let Some(ref player) = *guard {
								player.sink.stop();
							}
							*guard = None;
						}
						player_state.set(PlayerState::Stopped);
						current_track_idx.set(None);
					},
					"⏹ Stop"
				}
			}
		}
	}
}

// ══════════════════════════════════════════════════════════════════════════════
// STYLES
// ══════════════════════════════════════════════════════════════════════════════

const CSS: &str = r#"
* {
	margin: 0;
	padding: 0;
	box-sizing: border-box;
}

body {
	font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
	background: #1a1a2e;
	color: #eee;
}

.app {
	display: flex;
	flex-direction: column;
	height: 100vh;
	padding: 16px;
	gap: 16px;
}

.header {
	text-align: center;
	font-size: 24px;
	font-weight: bold;
	color: #00d4ff;
	padding: 12px;
	border-bottom: 1px solid #333;
}

.cassette-icon {
	color: #ffcc00;
}

.track-list {
	flex: 1;
	overflow-y: auto;
	border: 1px solid #333;
	border-radius: 8px;
	background: #16213e;
}

.track {
	display: flex;
	align-items: center;
	padding: 12px 16px;
	gap: 12px;
	cursor: pointer;
	border-bottom: 1px solid #2a2a4a;
	transition: background 0.15s;
}

.track:hover {
	background: #1f3460;
}

.track.selected {
	background: #0f4c81;
}

.track.playing {
	background: #1e5128;
}

.track-number {
	color: #888;
	min-width: 24px;
}

.track-info {
	flex: 1;
}

.track-title {
	font-weight: 500;
	color: #fff;
}

.track-artist {
	font-size: 12px;
	color: #888;
	margin-top: 2px;
}

.track-duration {
	color: #666;
	font-size: 14px;
}

.now-playing {
	background: #16213e;
	border: 1px solid #333;
	border-radius: 8px;
	padding: 16px;
	text-align: center;
}

.np-info {
	margin-bottom: 8px;
}

.np-title {
	font-size: 18px;
	font-weight: bold;
	color: #00d4ff;
}

.np-artist {
	color: #888;
}

.np-state {
	color: #4ade80;
	font-size: 14px;
}

.np-empty {
	color: #666;
	font-style: italic;
}

.controls {
	display: flex;
	justify-content: center;
	gap: 12px;
}

.controls button {
	background: #0f4c81;
	color: #fff;
	border: none;
	padding: 12px 24px;
	border-radius: 8px;
	cursor: pointer;
	font-size: 16px;
	transition: background 0.15s;
}

.controls button:hover {
	background: #1565c0;
}
"#;
