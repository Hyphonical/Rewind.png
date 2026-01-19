// ══════════════════════════════════════════════════════════════════════════════
// TUI MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Interactive terminal user interface for Rewind.png cassettes. Renders a
// skeuomorphic cassette player with clickable buttons, volume control, and
// a scrolling playlist. Fixed-size design inspired by vintage tape players.

use std::io::{self, Read, Seek, SeekFrom, Cursor};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering}};
use std::time::Duration;
use std::thread;
use std::fs::OpenOptions;

use crossterm::{
	event::{self, Event, KeyCode, KeyEventKind, MouseEvent, MouseEventKind, MouseButton, EnableMouseCapture, DisableMouseCapture},
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
	ExecutableCommand,
};
use ratatui::{
	backend::CrosstermBackend,
	layout::Rect,
	style::{Color, Modifier, Style},
	text::{Line, Span},
	widgets::Paragraph,
	Frame, Terminal,
};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;

use crate::io::{open_file, find_iend, format_duration};

// ══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ══════════════════════════════════════════════════════════════════════════════

const UI_WIDTH: u16 = 64;
const MAX_PLAYLIST_VISIBLE: usize = 5;

// Button positions (x, y, width) - Y is the row with the button icons
const BTN_PREV: (u16, u16, u16) = (7, 12, 5);
const BTN_PLAY: (u16, u16, u16) = (13, 12, 5);
const BTN_PAUSE: (u16, u16, u16) = (19, 12, 5);
const BTN_STOP: (u16, u16, u16) = (25, 12, 5);
const BTN_NEXT: (u16, u16, u16) = (31, 12, 5);
const BTN_VOL_DOWN: (u16, u16, u16) = (45, 12, 5);
const BTN_VOL_UP: (u16, u16, u16) = (51, 12, 5);

// Playlist first item Y position
const PLAYLIST_START_Y: u16 = 17;

// ══════════════════════════════════════════════════════════════════════════════
// DATA STRUCTURES
// ══════════════════════════════════════════════════════════════════════════════

/// Represents a single audio track on the cassette
#[allow(dead_code)]
pub struct Track {
	pub name: String,
	pub size: u64,
	pub offset: u64,
	pub artist: String,
	pub title: String,
	pub duration_secs: u64,
}

/// Player state
#[derive(PartialEq, Clone, Copy)]
pub enum PlayerState {
	Stopped,
	Playing,
	Paused,
}

/// Main application state
pub struct App {
	pub cassette_path: String,
	pub tracks: Vec<Track>,
	pub selected_track: usize,
	pub player_state: PlayerState,
	pub current_track: Option<usize>,
	pub progress_secs: Arc<AtomicU64>,
	pub should_quit: bool,
	pub stream: Option<OutputStream>,
	pub sink: Option<Sink>,
	pub is_playing: Arc<AtomicBool>,
	pub is_paused: Arc<AtomicBool>,
	pub playback_generation: Arc<AtomicU64>,
	pub volume: Arc<AtomicU8>, // 0-4 (maps to 0%, 25%, 50%, 75%, 100%)
	pub playlist_scroll: usize,
}

impl App {
	/// Creates a new App from a cassette file path
	pub fn new(cassette_path: &str) -> Result<Self, String> {
		let tracks = load_tracks(cassette_path)?;
		if tracks.is_empty() {
			return Err("This cassette is blank. No tracks found.".to_string());
		}

		Ok(App {
			cassette_path: cassette_path.to_string(),
			tracks,
			selected_track: 0,
			player_state: PlayerState::Stopped,
			current_track: None,
			progress_secs: Arc::new(AtomicU64::new(0)),
			should_quit: false,
			stream: None,
			sink: None,
			is_playing: Arc::new(AtomicBool::new(false)),
			is_paused: Arc::new(AtomicBool::new(false)),
			playback_generation: Arc::new(AtomicU64::new(0)),
			volume: Arc::new(AtomicU8::new(4)), // Start at 100%
			playlist_scroll: 0,
		})
	}

	/// Move selection up
	pub fn select_previous(&mut self) {
		if self.tracks.is_empty() { return; }
		self.selected_track = if self.selected_track == 0 {
			self.tracks.len() - 1
		} else {
			self.selected_track - 1
		};
		self.update_scroll();
	}

	/// Move selection down
	pub fn select_next(&mut self) {
		if self.tracks.is_empty() { return; }
		self.selected_track = if self.selected_track >= self.tracks.len() - 1 {
			0
		} else {
			self.selected_track + 1
		};
		self.update_scroll();
	}

	/// Update scroll position to keep selection visible
	fn update_scroll(&mut self) {
		let visible = self.tracks.len().min(MAX_PLAYLIST_VISIBLE);
		if self.selected_track < self.playlist_scroll {
			self.playlist_scroll = self.selected_track;
		} else if self.selected_track >= self.playlist_scroll + visible {
			self.playlist_scroll = self.selected_track - visible + 1;
		}
	}

	/// Select track by index (for mouse clicks)
	pub fn select_track(&mut self, idx: usize) {
		if idx < self.tracks.len() {
			self.selected_track = idx;
		}
	}

	/// Play the currently selected track
	pub fn play_selected(&mut self) {
		self.play_track(self.selected_track);
	}

	/// Play a specific track
	pub fn play_track(&mut self, idx: usize) {
		if idx >= self.tracks.len() { return; }
		self.stop_internal();

		self.current_track = Some(idx);
		self.selected_track = idx;
		self.update_scroll();
		self.progress_secs.store(0, Ordering::SeqCst);

		// Get track info before borrowing for load
		let cassette_path = self.cassette_path.clone();
		let track_offset = self.tracks[idx].offset;
		let track_size = self.tracks[idx].size;
		let track_duration = self.tracks[idx].duration_secs;

		// Load audio data
		let audio_data = match load_track_data_raw(&cassette_path, track_offset, track_size) {
			Ok(data) => data,
			Err(_) => return,
		};

		// Set up audio output
		let stream_handle = match OutputStreamBuilder::open_default_stream() {
			Ok(s) => s,
			Err(_) => return,
		};

		let sink = Sink::connect_new(&stream_handle.mixer());
		sink.set_volume(self.get_volume_float());

		let cursor = Cursor::new(audio_data);
		let source = match Decoder::new(cursor) {
			Ok(s) => s,
			Err(_) => return,
		};

		sink.append(source);
		self.stream = Some(stream_handle);
		self.sink = Some(sink);
		self.player_state = PlayerState::Playing;
		self.is_playing.store(true, Ordering::SeqCst);
		self.is_paused.store(false, Ordering::SeqCst);

		// Start progress tracker
		let new_gen = self.playback_generation.fetch_add(1, Ordering::SeqCst) + 1;
		let progress = Arc::clone(&self.progress_secs);
		let is_playing = Arc::clone(&self.is_playing);
		let is_paused = Arc::clone(&self.is_paused);
		let generation = Arc::clone(&self.playback_generation);
		let duration = track_duration;

		thread::spawn(move || {
			let mut elapsed = 0u64;
			while is_playing.load(Ordering::SeqCst) {
				if generation.load(Ordering::SeqCst) != new_gen { break; }
				if elapsed >= duration { break; }
				if !is_paused.load(Ordering::SeqCst) {
					elapsed += 1;
					progress.store(elapsed, Ordering::SeqCst);
				}
				thread::sleep(Duration::from_secs(1));
			}
		});
	}

	/// Toggle pause/resume
	pub fn toggle_pause(&mut self) {
		match self.player_state {
			PlayerState::Playing => {
				if let Some(ref sink) = self.sink {
					sink.pause();
					self.is_paused.store(true, Ordering::SeqCst);
					self.player_state = PlayerState::Paused;
				}
			}
			PlayerState::Paused => {
				if let Some(ref sink) = self.sink {
					sink.play();
					self.is_paused.store(false, Ordering::SeqCst);
					self.player_state = PlayerState::Playing;
				}
			}
			PlayerState::Stopped => self.play_selected(),
		}
	}

	/// Stop playback (internal, doesn't reset current_track for display)
	fn stop_internal(&mut self) {
		self.is_playing.store(false, Ordering::SeqCst);
		if let Some(sink) = self.sink.take() {
			sink.stop();
		}
		self.stream = None;
	}

	/// Stop playback completely
	pub fn stop(&mut self) {
		self.stop_internal();
		self.player_state = PlayerState::Stopped;
		self.current_track = None;
		self.progress_secs.store(0, Ordering::SeqCst);
	}

	/// Skip to next track
	pub fn next_track(&mut self) {
		let next = match self.current_track {
			Some(idx) => if idx >= self.tracks.len() - 1 { 0 } else { idx + 1 },
			None => self.selected_track,
		};
		self.play_track(next);
	}

	/// Skip to previous track
	pub fn previous_track(&mut self) {
		let prev = match self.current_track {
			Some(idx) => if idx == 0 { self.tracks.len() - 1 } else { idx - 1 },
			None => self.selected_track,
		};
		self.play_track(prev);
	}

	/// Increase volume
	pub fn volume_up(&mut self) {
		let current = self.volume.load(Ordering::SeqCst);
		if current < 4 {
			self.volume.store(current + 1, Ordering::SeqCst);
			self.apply_volume();
		}
	}

	/// Decrease volume
	pub fn volume_down(&mut self) {
		let current = self.volume.load(Ordering::SeqCst);
		if current > 0 {
			self.volume.store(current - 1, Ordering::SeqCst);
			self.apply_volume();
		}
	}

	/// Get volume as float (0.0 - 1.0)
	fn get_volume_float(&self) -> f32 {
		self.volume.load(Ordering::SeqCst) as f32 * 0.25
	}

	/// Apply volume to active sink
	fn apply_volume(&mut self) {
		if let Some(ref sink) = self.sink {
			sink.set_volume(self.get_volume_float());
		}
	}

	/// Check if current track finished, auto-advance
	pub fn check_track_finished(&mut self) {
		if let Some(ref sink) = self.sink {
			if sink.empty() && self.player_state == PlayerState::Playing {
				if let Some(idx) = self.current_track {
					if idx < self.tracks.len() - 1 {
						self.next_track();
					} else {
						self.stop();
					}
				}
			}
		}
	}
}

// ══════════════════════════════════════════════════════════════════════════════
// CASSETTE LOADING
// ══════════════════════════════════════════════════════════════════════════════

/// Load track metadata from a cassette file
fn load_tracks(path: &str) -> Result<Vec<Track>, String> {
	let mut file = open_file(path)?;
	let toc_pos = find_iend(&mut file)
		.ok_or("This cassette appears to be blank. No IEND chunk found.")?;

	file.seek(SeekFrom::Start(toc_pos)).map_err(|e| e.to_string())?;

	let mut count_buf = [0u8; 4];
	file.read_exact(&mut count_buf).map_err(|e| e.to_string())?;
	let track_count = u32::from_le_bytes(count_buf) as usize;

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

	let audio_start = file.stream_position().map_err(|e| e.to_string())?;
	let mut tracks = Vec::new();
	let mut offset = audio_start;

	for (name, size) in entries {
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

		tracks.push(Track { name, size, offset, artist, title, duration_secs });
		offset += size;
	}

	Ok(tracks)
}

/// Load raw audio data by offset and size
fn load_track_data_raw(cassette_path: &str, offset: u64, size: u64) -> Result<Vec<u8>, String> {
	let mut file = open_file(cassette_path)?;
	file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
	let mut data = vec![0u8; size as usize];
	file.read_exact(&mut data).map_err(|e| e.to_string())?;
	Ok(data)
}

// ══════════════════════════════════════════════════════════════════════════════
// TERMINAL UI
// ══════════════════════════════════════════════════════════════════════════════

/// Main entry point for the TUI
pub fn run_tui(cassette_path: &str) -> Result<(), String> {
	// Suppress stderr (rodio messages)
	#[cfg(windows)]
	let _stderr_redirect = OpenOptions::new().write(true).open("NUL")
		.ok().and_then(|f| gag::Redirect::stderr(f).ok());
	#[cfg(not(windows))]
	let _stderr_redirect = OpenOptions::new().write(true).open("/dev/null")
		.ok().and_then(|f| gag::Redirect::stderr(f).ok());

	let mut app = App::new(cassette_path)?;

	enable_raw_mode().map_err(|e| e.to_string())?;
	let mut stdout = io::stdout();
	stdout.execute(EnterAlternateScreen).map_err(|e| e.to_string())?;
	stdout.execute(EnableMouseCapture).map_err(|e| e.to_string())?;

	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

	let result = run_app(&mut terminal, &mut app);

	disable_raw_mode().map_err(|e| e.to_string())?;
	io::stdout().execute(LeaveAlternateScreen).map_err(|e| e.to_string())?;
	io::stdout().execute(DisableMouseCapture).map_err(|e| e.to_string())?;

	result
}

/// Check if a click is within a button area
fn is_click_in_button(x: u16, y: u16, btn: (u16, u16, u16), ui_x: u16, ui_y: u16) -> bool {
	let bx = ui_x + btn.0;
	let by = ui_y + btn.1;
	x >= bx && x < bx + btn.2 && y == by
}

/// Check if a click is on a playlist item, returns track index if so
fn get_playlist_click(x: u16, y: u16, ui_x: u16, ui_y: u16, scroll: usize, track_count: usize) -> Option<usize> {
	let playlist_x_start = ui_x + 3;
	let playlist_x_end = ui_x + 60;
	let visible = track_count.min(MAX_PLAYLIST_VISIBLE);
	
	for i in 0..visible {
		let item_y = ui_y + PLAYLIST_START_Y + i as u16;
		if y == item_y && x >= playlist_x_start && x < playlist_x_end {
			let track_idx = scroll + i;
			if track_idx < track_count {
				return Some(track_idx);
			}
		}
	}
	None
}

/// Main application loop
fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<(), String> {
	// Calculate UI position (centered or top-left)
	let ui_x: u16 = 0;
	let ui_y: u16 = 0;

	loop {
		app.check_track_finished();
		terminal.draw(|f| draw_ui(f, app)).map_err(|e| e.to_string())?;

		if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
			match event::read().map_err(|e| e.to_string())? {
				Event::Key(key) if key.kind == KeyEventKind::Press => {
					match key.code {
						KeyCode::Char('q') | KeyCode::Esc => {
							app.stop();
							app.should_quit = true;
						}
						KeyCode::Up | KeyCode::Char('k') => app.select_previous(),
						KeyCode::Down | KeyCode::Char('j') => app.select_next(),
						KeyCode::Enter => app.play_selected(),
						KeyCode::Char(' ') => app.toggle_pause(),
						KeyCode::Char('s') => app.stop(),
						KeyCode::Right | KeyCode::Char('n') => app.next_track(),
						KeyCode::Left | KeyCode::Char('p') => app.previous_track(),
						KeyCode::Char('+') | KeyCode::Char('=') => app.volume_up(),
						KeyCode::Char('-') => app.volume_down(),
						_ => {}
					}
				}
				Event::Mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, .. }) => {
					// Check button clicks
					if is_click_in_button(column, row, BTN_PREV, ui_x, ui_y) {
						app.previous_track();
					} else if is_click_in_button(column, row, BTN_PLAY, ui_x, ui_y) {
						if app.player_state == PlayerState::Stopped {
							app.play_selected();
						} else if app.player_state == PlayerState::Paused {
							app.toggle_pause();
						}
					} else if is_click_in_button(column, row, BTN_PAUSE, ui_x, ui_y) {
						if app.player_state == PlayerState::Playing {
							app.toggle_pause();
						}
					} else if is_click_in_button(column, row, BTN_STOP, ui_x, ui_y) {
						app.stop();
					} else if is_click_in_button(column, row, BTN_NEXT, ui_x, ui_y) {
						app.next_track();
					} else if is_click_in_button(column, row, BTN_VOL_DOWN, ui_x, ui_y) {
						app.volume_down();
					} else if is_click_in_button(column, row, BTN_VOL_UP, ui_x, ui_y) {
						app.volume_up();
					} else if let Some(track_idx) = get_playlist_click(column, row, ui_x, ui_y, app.playlist_scroll, app.tracks.len()) {
						app.select_track(track_idx);
						app.play_track(track_idx);
					}
				}
				_ => {}
			}
		}

		if app.should_quit { break; }
	}

	Ok(())
}

// ══════════════════════════════════════════════════════════════════════════════
// DRAWING
// ══════════════════════════════════════════════════════════════════════════════

/// Draw the complete cassette player UI
fn draw_ui(f: &mut Frame, app: &App) {
	let mut lines: Vec<Line> = Vec::new();
	
	let volume = app.volume.load(Ordering::SeqCst);
	let (elapsed, duration, progress_ratio) = if let Some(idx) = app.current_track {
		let track = &app.tracks[idx];
		let e = app.progress_secs.load(Ordering::SeqCst);
		let d = track.duration_secs.max(1);
		(e, d, (e as f64 / d as f64).min(1.0))
	} else {
		(0, 0, 0.0)
	};

	// Get current track info
	let (artist_title, track_num_str) = if let Some(idx) = app.current_track {
		let track = &app.tracks[idx];
		let display = format!("{} - {}", track.artist, track.title);
		let num = format!("[{}/{}]", idx + 1, app.tracks.len());
		(display, num)
	} else {
		("No track loaded".to_string(), format!("[-/{}]", app.tracks.len()))
	};

	// Truncate artist_title to fit (max 24 chars)
	let max_title_len = 24;
	let artist_title_display: String = if artist_title.chars().count() > max_title_len {
		artist_title.chars().take(max_title_len - 1).collect::<String>() + "…"
	} else {
		format!("{:<24}", artist_title)
	};

	// Build progress bar (24 chars wide)
	let progress_width = 24;
	let filled = (progress_ratio * progress_width as f64) as usize;
	let empty = progress_width - filled;
	let progress_bar = format!("{}{}", "═".repeat(filled), "╌".repeat(empty));

	// Time display
	let time_str = format!("{} / {}", format_duration(elapsed), format_duration(duration));
	let time_display = format!("{:<13} {:>7}", time_str, track_num_str);

	// Dynamic playlist size
	let playlist_visible = app.tracks.len().min(MAX_PLAYLIST_VISIBLE);

	// Volume slider: knob ╞══╡ at volume level, bar ├──┤ one position above
	// At volume 0: only bar at bottom, no knob visible
	let vol_slot = |pos: u8| -> &str {
		if volume == 0 {
			if pos == 0 { "├──┤" } else { "│  │" }
		} else if pos == volume {
			"╞══╡"
		} else if pos == volume + 1 && pos <= 4 {
			"├──┤"
		} else {
			"│  │"
		}
	};

	// Line 0: Top cassette border
	lines.push(Line::from("       ╭──────────────────────────────────────────────╮"));
	// Line 1: Cassette shell top
	lines.push(Line::from("╭──────┤                                              ├──────╮"));
	// Line 2: Brand + left reel + title + right reel + volume top
	lines.push(Line::from(vec![
		Span::raw("│ "),
		Span::styled("Sony", Style::default().fg(Color::Yellow)),
		Span::raw(" │   ╭─────╮ "),
		Span::styled(artist_title_display.clone(), Style::default().fg(Color::Cyan)),
		Span::raw(" ╭─────╮   │ ╭──╮ │"),
	]));
	// Line 3: Reels inner + progress bar + volume slot 4
	lines.push(Line::from(vec![
		Span::raw("│      │   │ ╭─╮ │ "),
		Span::styled(progress_bar.clone(), Style::default().fg(Color::Green)),
		Span::raw(" │ ╭─╮ │   │ "),
		Span::raw(vol_slot(4)),
		Span::raw(" │"),
	]));
	// Line 4: Reels + time display + volume slot 3
	lines.push(Line::from(vec![
		Span::raw("│      │   │ ╰─╯ │ "),
		Span::styled(format!("{:<24}", time_display), Style::default().fg(Color::White)),
		Span::raw(" │ ╰─╯ │   │ "),
		Span::raw(vol_slot(3)),
		Span::raw(" │"),
	]));
	// Line 5: Reel bottom + volume slot 2
	lines.push(Line::from(vec![
		Span::raw("│      │   ╰─────╯                          ╰─────╯   │ "),
		Span::raw(vol_slot(2)),
		Span::raw(" │"),
	]));
	// Line 6: Cassette body + volume slot 1
	lines.push(Line::from(vec![
		Span::raw("│      │                                              │ "),
		Span::raw(vol_slot(1)),
		Span::raw(" │"),
	]));
	// Line 7: Tape window + volume slot 0
	lines.push(Line::from(vec![
		Span::raw("│      │   ╒══════════════════════════════════════╕   │ "),
		Span::raw(vol_slot(0)),
		Span::raw(" │"),
	]));
	// Line 8: Cassette inner border + volume bottom
	lines.push(Line::from("│      ├───┴──────────────────────────────────────┴───┤ ╰──╯ │"));
	// Line 9: Cassette bottom + volume percentage
	lines.push(Line::from(format!("│      ╰──────────────────────────────────────────────╯ {:>3}% │", volume * 25)));
	// Line 10: Empty line before buttons
	lines.push(Line::from("│                                                            │"));
	// Line 11: Button tops
	lines.push(Line::from("│      ┌───┐ ┌───┐ ┌───┐ ┌───┐ ┌───┐    │   ┌───┐ ┌───┐      │"));
	// Line 12: Button icons
	let play_style = if app.player_state == PlayerState::Playing { Style::default().fg(Color::Green) } else { Style::default() };
	let pause_style = if app.player_state == PlayerState::Paused { Style::default().fg(Color::Yellow) } else { Style::default() };
	let stop_style = if app.player_state == PlayerState::Stopped { Style::default().fg(Color::Red) } else { Style::default() };
	
	lines.push(Line::from(vec![
		Span::raw("│      │ "),
		Span::styled("⏮", Style::default()),
		Span::raw(" │ │ "),
		Span::styled("▶", play_style),
		Span::raw(" │ │ "),
		Span::styled("⏸", pause_style),
		Span::raw(" │ │ "),
		Span::styled("■", stop_style),
		Span::raw(" │ │ "),
		Span::styled("⏭", Style::default()),
		Span::raw(" │    │   │ "),
		Span::styled("-", Style::default()),
		Span::raw(" │ │ "),
		Span::styled("+", Style::default()),
		Span::raw(" │      │"),
	]));
	// Line 13: Button bottoms
	lines.push(Line::from("│      ╘═══╛ ╘═══╛ ╘═══╛ ╘═══╛ ╘═══╛    │   ╘═══╛ ╘═══╛      │"));
	// Line 14: Button labels
	lines.push(Line::from("│      Prev  Play  Pause Stop  Next             Vol          │"));
	lines.push(Line::from("│                                                            │"));
	// Line 15: Playlist header - centered (54 char inner box)
	lines.push(Line::from("│    ┌─ PLAYLIST ───────────────────────────────────────┐    │"));

	// Playlist items (dynamic based on track count)
	for i in 0..playlist_visible {
		let track_idx = app.playlist_scroll + i;
		if track_idx < app.tracks.len() {
			let track = &app.tracks[track_idx];
			let is_current = app.current_track == Some(track_idx);
			let is_selected = app.selected_track == track_idx;
			
			let prefix = if is_current {
				match app.player_state {
					PlayerState::Playing => "▶",
					PlayerState::Paused => "⏸",
					PlayerState::Stopped => " ",
				}
			} else if is_selected {
				">"
			} else {
				" "
			};

			// Format track: keep duration visible, truncate name more aggressively
			let duration_str = format!("[{}]", format_duration(track.duration_secs));
			let num_prefix = format!("{:2}. ", track_idx + 1);
			let name_part = format!("{} - {}", track.artist, track.title);
			
			// Content width: 46 chars to fit properly (shifted 4 left)
			// = num_prefix(4) + name + space(1) + duration(~6)
			let content_width = 46;
			let available = content_width - num_prefix.len() - duration_str.len() - 1;
			let name_display: String = if name_part.chars().count() > available {
				name_part.chars().take(available - 1).collect::<String>() + "…"
			} else {
				format!("{:<width$}", name_part, width = available)
			};
			
			let content = format!("{}{} {}", num_prefix, name_display, duration_str);

			let style = if is_current {
				Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
			} else if is_selected {
				Style::default().fg(Color::Yellow)
			} else {
				Style::default()
			};

			lines.push(Line::from(vec![
				Span::raw("│    │ "),
				Span::styled(prefix, style),
				Span::raw(" "),
				Span::styled(content, style),
				Span::raw(" │    │"),
			]));
		}
	}

	// Playlist bottom - centered to match header
	lines.push(Line::from("│    ╘══════════════════════════════════════════════════╛    │"));
	// Separator
	lines.push(Line::from("├────────────────────────────────────────────────────────────┤"));
	// Controls hint
	lines.push(Line::from(vec![
		Span::raw("│ "),
		Span::styled("[⇅]", Style::default().fg(Color::Yellow)),
		Span::raw(" Navigate  "),
		Span::styled("[Enter]", Style::default().fg(Color::Yellow)),
		Span::raw(" Select  "),
		Span::styled("[Space]", Style::default().fg(Color::Yellow)),
		Span::raw(" Play/Pause  "),
		Span::styled("[Q]", Style::default().fg(Color::Yellow)),
		Span::raw(" Quit │"),
	]));
	// Bottom border
	lines.push(Line::from("╰────────────────────────────────────────────────────────────╯"));

	let total_height = lines.len() as u16;
	let paragraph = Paragraph::new(lines);
	f.render_widget(paragraph, Rect::new(0, 0, UI_WIDTH, total_height));
}
