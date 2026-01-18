// ══════════════════════════════════════════════════════════════════════════════
// TUI MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Interactive terminal user interface for Rewind.png cassettes. Provides a
// visual track list, playback controls, and progress display using ratatui.
// Designed to feel like operating a real cassette player.

use std::io::{self, Read, Seek, SeekFrom, Cursor};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use std::time::Duration;
use std::thread;
use std::fs::OpenOptions;

use crossterm::{
	event::{self, Event, KeyCode, KeyEventKind},
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
	ExecutableCommand,
};
use ratatui::{
	backend::CrosstermBackend,
	layout::{Alignment, Constraint, Layout, Rect},
	style::{Color, Modifier, Style},
	text::{Line, Span},
	widgets::{Block, Borders, Gauge, List, ListItem, ListState, Paragraph},
	Frame, Terminal,
};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;

use crate::io::{open_file, find_iend, format_duration};

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

/// Player state for the TUI
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
	pub list_state: ListState,
	pub player_state: PlayerState,
	pub current_track: Option<usize>,
	pub progress_secs: Arc<AtomicU64>,
	pub should_quit: bool,
	pub stream: Option<OutputStream>,  // Keep stream alive!
	pub sink: Option<Sink>,
	pub is_playing: Arc<AtomicBool>,
	pub is_paused: Arc<AtomicBool>,
	pub playback_generation: Arc<AtomicU64>,  // Increments each play, old threads check this
	pub status_message: String,
}

impl App {
	/// Creates a new App from a cassette file path
	pub fn new(cassette_path: &str) -> Result<Self, String> {
		let tracks = load_tracks(cassette_path)?;

		if tracks.is_empty() {
			return Err("This cassette is blank. No tracks found.".to_string());
		}

		let mut list_state = ListState::default();
		list_state.select(Some(0));

		Ok(App {
			cassette_path: cassette_path.to_string(),
			tracks,
			list_state,
			player_state: PlayerState::Stopped,
			current_track: None,
			progress_secs: Arc::new(AtomicU64::new(0)),
			should_quit: false,
			stream: None,
			sink: None,
			is_playing: Arc::new(AtomicBool::new(false)),
			is_paused: Arc::new(AtomicBool::new(false)),
			playback_generation: Arc::new(AtomicU64::new(0)),
			status_message: "Ready. Press Enter to play, Q to quit.".to_string(),
		})
	}

	/// Move selection up in the track list
	pub fn select_previous(&mut self) {
		if self.tracks.is_empty() { return; }
		let i = match self.list_state.selected() {
			Some(i) => if i == 0 { self.tracks.len() - 1 } else { i - 1 },
			None => 0,
		};
		self.list_state.select(Some(i));
	}

	/// Move selection down in the track list
	pub fn select_next(&mut self) {
		if self.tracks.is_empty() { return; }
		let i = match self.list_state.selected() {
			Some(i) => if i >= self.tracks.len() - 1 { 0 } else { i + 1 },
			None => 0,
		};
		self.list_state.select(Some(i));
	}

	/// Start playing the currently selected track
	pub fn play_selected(&mut self) {
		if let Some(idx) = self.list_state.selected() {
			self.play_track(idx);
		}
	}

	/// Play a specific track by index
	pub fn play_track(&mut self, idx: usize) {
		// Stop any existing playback
		self.stop();

		let track = &self.tracks[idx];
		self.current_track = Some(idx);
		self.progress_secs.store(0, Ordering::SeqCst);

		// Load audio data from cassette
		let audio_data = match load_track_data(&self.cassette_path, track) {
			Ok(data) => data,
			Err(e) => {
				self.status_message = format!("Error loading track: {}", e);
				return;
			}
		};

		// Set up audio output
		let stream_handle = match OutputStreamBuilder::open_default_stream() {
			Ok(s) => s,
			Err(e) => {
				self.status_message = format!("Audio device error: {}", e);
				return;
			}
		};

		let sink = Sink::connect_new(&stream_handle.mixer());

		let cursor = Cursor::new(audio_data);
		let source = match Decoder::new(cursor) {
			Ok(s) => s,
			Err(e) => {
				self.status_message = format!("Cannot decode track: {}", e);
				return;
			}
		};

		sink.append(source);
		self.stream = Some(stream_handle);  // Keep stream alive!
		self.sink = Some(sink);
		self.player_state = PlayerState::Playing;
		self.is_playing.store(true, Ordering::SeqCst);
		self.is_paused.store(false, Ordering::SeqCst);

		// Increment generation so old progress threads know to exit
		let new_gen = self.playback_generation.fetch_add(1, Ordering::SeqCst) + 1;

		// Start progress tracker in background
		let progress = Arc::clone(&self.progress_secs);
		let is_playing = Arc::clone(&self.is_playing);
		let is_paused = Arc::clone(&self.is_paused);
		let generation = Arc::clone(&self.playback_generation);
		let duration = track.duration_secs;

		thread::spawn(move || {
			let mut elapsed_secs = 0u64;
			while is_playing.load(Ordering::SeqCst) {
				// Exit if a newer playback started
				if generation.load(Ordering::SeqCst) != new_gen {
					break;
				}
				if elapsed_secs >= duration {
					break;
				}
				// Only increment when not paused
				if !is_paused.load(Ordering::SeqCst) {
					elapsed_secs += 1;
					progress.store(elapsed_secs, Ordering::SeqCst);
				}
				thread::sleep(Duration::from_secs(1));
			}
		});

		self.status_message = format!("▶ Playing: {} - {}", track.artist, track.title);
	}

	/// Toggle pause/resume
	pub fn toggle_pause(&mut self) {
		if let Some(ref sink) = self.sink {
			match self.player_state {
				PlayerState::Playing => {
					sink.pause();
					self.is_paused.store(true, Ordering::SeqCst);
					self.player_state = PlayerState::Paused;
					self.status_message = "⏸ Paused".to_string();
				}
				PlayerState::Paused => {
					sink.play();
					self.is_paused.store(false, Ordering::SeqCst);
					self.player_state = PlayerState::Playing;
					if let Some(idx) = self.current_track {
						let track = &self.tracks[idx];
						self.status_message = format!("▶ Playing: {} - {}", track.artist, track.title);
					}
				}
				PlayerState::Stopped => {
					self.play_selected();
				}
			}
		} else {
			self.play_selected();
		}
	}

	/// Stop playback completely
	pub fn stop(&mut self) {
		self.is_playing.store(false, Ordering::SeqCst);
		if let Some(sink) = self.sink.take() {
			sink.stop();
		}
		self.stream = None;  // Drop the stream after sink
		self.player_state = PlayerState::Stopped;
		self.current_track = None;
		self.progress_secs.store(0, Ordering::SeqCst);
		self.status_message = "■ Stopped".to_string();
	}

	/// Skip to next track
	pub fn next_track(&mut self) {
		if let Some(idx) = self.current_track {
			let next = if idx >= self.tracks.len() - 1 { 0 } else { idx + 1 };
			self.list_state.select(Some(next));
			self.play_track(next);
		} else {
			self.select_next();
			self.play_selected();
		}
	}

	/// Skip to previous track
	pub fn previous_track(&mut self) {
		if let Some(idx) = self.current_track {
			let prev = if idx == 0 { self.tracks.len() - 1 } else { idx - 1 };
			self.list_state.select(Some(prev));
			self.play_track(prev);
		} else {
			self.select_previous();
			self.play_selected();
		}
	}

	/// Check if current track has finished, auto-advance to next
	pub fn check_track_finished(&mut self) {
		if let Some(ref sink) = self.sink {
			if sink.empty() && self.player_state == PlayerState::Playing {
				// Track finished, play next
				if let Some(idx) = self.current_track {
					if idx < self.tracks.len() - 1 {
						self.next_track();
					} else {
						// Last track finished
						self.stop();
						self.status_message = "✔ Cassette complete!".to_string();
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
		// Read track data to extract metadata
		file.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
		let mut audio_data = vec![0u8; size as usize];
		file.read_exact(&mut audio_data).map_err(|e| e.to_string())?;

		// Extract metadata using lofty
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
// TERMINAL UI
// ══════════════════════════════════════════════════════════════════════════════

/// Main entry point for the TUI
pub fn run_tui(cassette_path: &str) -> Result<(), String> {
	// Redirect stderr to /dev/null to suppress rodio's "Dropping OutputStream" messages
	#[cfg(windows)]
	let _stderr_redirect = OpenOptions::new().write(true).open("NUL")
		.ok()
		.and_then(|f| gag::Redirect::stderr(f).ok());

	#[cfg(not(windows))]
	let _stderr_redirect = OpenOptions::new().write(true).open("/dev/null")
		.ok()
		.and_then(|f| gag::Redirect::stderr(f).ok());

	// Initialize app
	let mut app = App::new(cassette_path)?;

	// Setup terminal
	enable_raw_mode().map_err(|e| e.to_string())?;
	io::stdout().execute(EnterAlternateScreen).map_err(|e| e.to_string())?;

	let backend = CrosstermBackend::new(io::stdout());
	let mut terminal = Terminal::new(backend).map_err(|e| e.to_string())?;

	// Main loop
	let result = run_app(&mut terminal, &mut app);

	// Cleanup
	disable_raw_mode().map_err(|e| e.to_string())?;
	io::stdout().execute(LeaveAlternateScreen).map_err(|e| e.to_string())?;

	result
}

/// Main application loop
fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<(), String> {
	loop {
		// Check if current track finished
		app.check_track_finished();

		// Draw UI
		terminal.draw(|f| draw_ui(f, app)).map_err(|e| e.to_string())?;

		// Handle input (non-blocking with timeout for responsiveness)
		if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
			if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
				if key.kind == KeyEventKind::Press {
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
						_ => {}
					}
				}
			}
		}

		if app.should_quit {
			break;
		}
	}

	Ok(())
}

/// Draw the complete UI
fn draw_ui(f: &mut Frame, app: &App) {
	let area = f.area();

	// Layout: Header (3) | Track List (flex) | Now Playing (6) | Controls (3)
	let chunks = Layout::vertical([
		Constraint::Length(3),   // Header
		Constraint::Min(5),      // Track list
		Constraint::Length(6),   // Now playing + progress
		Constraint::Length(3),   // Controls
	]).split(area);

	draw_header(f, chunks[0]);
	draw_track_list(f, chunks[1], app);
	draw_now_playing(f, chunks[2], app);
	draw_controls(f, chunks[3], app);
}

/// Draw the header with cassette name
fn draw_header(f: &mut Frame, area: Rect) {
	let header = Paragraph::new(Line::from(vec![
		Span::styled(" ◉ ", Style::default().fg(Color::Red)),
		Span::styled("REWIND", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
		Span::styled(".PNG ", Style::default().fg(Color::White)),
		Span::styled("[●▪▪●]", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
	]))
	.alignment(Alignment::Center)
	.block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)));

	f.render_widget(header, area);
}

/// Draw the track list
fn draw_track_list(f: &mut Frame, area: Rect, app: &App) {
	let items: Vec<ListItem> = app.tracks.iter().enumerate().map(|(i, track)| {
		let is_current = app.current_track == Some(i);
		let is_selected = app.list_state.selected() == Some(i);

		let prefix = if is_current {
			match app.player_state {
				PlayerState::Playing => "▶ ",
				PlayerState::Paused => "⏸ ",
				PlayerState::Stopped => "  ",
			}
		} else {
			"  "
		};

		let content = format!(
			"{}{:2}. {} - {} [{}]",
			prefix,
			i + 1,
			track.artist,
			track.title,
			format_duration(track.duration_secs)
		);

		let style = if is_current {
			Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
		} else if is_selected {
			Style::default().fg(Color::Yellow)
		} else {
			Style::default().fg(Color::White)
		};

		ListItem::new(content).style(style)
	}).collect();

	let list = List::new(items)
		.block(Block::default()
			.title(" ♫ Tracks ")
			.borders(Borders::ALL)
			.border_style(Style::default().fg(Color::DarkGray)))
		.highlight_style(Style::default().bg(Color::DarkGray))
		.highlight_symbol("› ");

	f.render_stateful_widget(list, area, &mut app.list_state.clone());
}

/// Draw the now playing section with progress bar
fn draw_now_playing(f: &mut Frame, area: Rect, app: &App) {
	let chunks = Layout::vertical([
		Constraint::Length(3),  // Track info
		Constraint::Length(3),  // Progress bar
	]).split(area);

	// Track info
	let info_text = if let Some(idx) = app.current_track {
		let track = &app.tracks[idx];
		format!("🎵 {} - {}", track.artist, track.title)
	} else {
		"No track selected".to_string()
	};

	let info = Paragraph::new(info_text)
		.alignment(Alignment::Center)
		.block(Block::default()
			.title(" Now Playing ")
			.borders(Borders::ALL)
			.border_style(Style::default().fg(Color::DarkGray)));

	f.render_widget(info, chunks[0]);

	// Progress bar
	let (progress_ratio, progress_text) = if let Some(idx) = app.current_track {
		let track = &app.tracks[idx];
		let elapsed = app.progress_secs.load(Ordering::SeqCst);
		let duration = track.duration_secs.max(1);
		let ratio = (elapsed as f64 / duration as f64).min(1.0);
		let text = format!("{} / {}", format_duration(elapsed), format_duration(duration));
		(ratio, text)
	} else {
		(0.0, "0:00 / 0:00".to_string())
	};

	let gauge = Gauge::default()
		.block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::DarkGray)))
		.gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
		.ratio(progress_ratio)
		.label(progress_text);

	f.render_widget(gauge, chunks[1]);
}

/// Draw the control hints
fn draw_controls(f: &mut Frame, area: Rect, app: &App) {
	let controls = Line::from(vec![
		Span::styled(" ↑↓ ", Style::default().fg(Color::Yellow)),
		Span::raw("Select  "),
		Span::styled(" Enter ", Style::default().fg(Color::Yellow)),
		Span::raw("Play  "),
		Span::styled(" Space ", Style::default().fg(Color::Yellow)),
		Span::raw("Pause  "),
		Span::styled(" ←→ ", Style::default().fg(Color::Yellow)),
		Span::raw("Prev/Next  "),
		Span::styled(" S ", Style::default().fg(Color::Yellow)),
		Span::raw("Stop  "),
		Span::styled(" Q ", Style::default().fg(Color::Yellow)),
		Span::raw("Quit"),
	]);

	let status = Paragraph::new(vec![
		controls,
		Line::from(Span::styled(&app.status_message, Style::default().fg(Color::DarkGray))),
	])
	.alignment(Alignment::Center)
	.block(Block::default().borders(Borders::TOP).border_style(Style::default().fg(Color::DarkGray)));

	f.render_widget(status, area);
}
