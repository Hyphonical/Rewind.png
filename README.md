# 🎵 Rewind.png

> **Digital cassette tapes disguised as PNG images**

Rewind.png is an experimental media format that embeds lossless audio (FLAC/MP3/OGG/WAV) inside PNG images without corrupting the visual data. Think of it as a modern cassette tape; The PNG is the box art, and the embedded audio is the music inside.

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Rust](https://img.shields.io/badge/rust-1.70+-orange)

---

## 🎯 Why?

**The ritual of physical media is dead.** Music streaming is convenient, but it's also ephemeral—tracks disappear, playlists break, and there's no *object* to treasure. Rewind.png brings back:

- **Tangible ownership**: One file = one album + its cover art
- **Intentional curation**: Like burning a mixtape, each cassette is deliberate
- **Fragility by design**: Re-compress the image? The audio breaks. This *forces* careful handling, just like vinyl records.

**Use cases:**
- Gift custom mixtapes with personalized cover art
- Archive music collections with embedded album art
- Share music in a nostalgic, tactile format

---

## 🛠️ How It Works

### Polyglot File Structure

| PNG Header Image Data (IDHR, IDAT Chunks) IEND Chunk          	|
|---------------------------------------------------------------	|
| Table of Contents (TOC)  - Track count  - Track names & sizes 	|
| Audio Track 1 (FLAC/MP3/OGG/WAV) Audio Track 2 ...            	|
| CRC32 Checksum (Integrity Seal)                               	|

**To image viewers:** It's just a normal PNG.  
**To Rewind:** It's a playable cassette with embedded audio.

### Security Model
- **Whitelist-only formats**: Only FLAC, MP3, OGG, and WAV are allowed
- **CRC32 integrity check**: If the file is modified (e.g., re-encoded by social media), playback is blocked with: *"This cassette has been damaged."*
- **Non-destructive embedding**: The PNG image remains fully viewable

---

## 📦 Installation

### Prerequisites
- **Rust 1.70+** ([Install Rust](https://rustup.rs/))
- **Audio files** (FLAC/MP3/OGG/WAV)
- **PNG image** (for cassette cover art)

### Build from Source
```bash
git clone https://github.com/Hyphonical/Rewind.png.git
cd Rewind.png
cargo build --release
```

Binary will be in `target/release/rewind`

---

## 🚀 Usage

### 1. Create a Cassette (Record)
Inject audio files into a PNG image:

```bash
rewind record cover.png track1.flac track2.flac track3.flac -o mixtape.png
```

**Output:** `mixtape.png` (viewable as image, playable as audio)

### 2. Inspect a Cassette
View embedded tracks and verify integrity:

```bash
rewind inspect mixtape.png
```

**Example output:**
```
[00:00:00] ✔  Checksum matches. The file is intact.
[00:00:00] 𝒊  TOC: 6 audio file(s)
[00:00:00] 𝒊    [1] Track_1.flac (10000000 bytes) | 🎵 Artist 1 - Track 1 [1:23]
[00:00:00] 𝒊    [2] Track_2.flac (10000000 bytes) | 🎵 Artist 2 - Track 2 [1:23]
...
```

### 3. Play a Cassette
Play a random track (for testing):

```bash
rewind play mixtape.png
```

**Output:**
```
[00:00:00] ✔  ▶ Now Playing: Artist 1 - Track 1 [1:23]
[00:00:00] 𝒊  Press Ctrl+C to stop.
```

---

## 🗺️ Roadmap

### ✅ Version 0.1 (Current)
- [x] Record audio into PNG images
- [x] Inspect cassette metadata & verify integrity
- [x] Random track playback (CLI-ish)

### 🚧 Version 0.2 (Next)
- [ ] CLI argument parsing (`clap`)
  - [ ] `rewind record <image> <audio...> -o <output>`
  - [ ] `rewind inspect <cassette>`
  - [ ] `rewind play <cassette> [--track N]`
- [ ] Sequential playback (play all tracks)
- [ ] Track extraction (`rewind extract <cassette> <track_number> -o output.flac`)
- [ ] Better error messages (user-friendly)

### 🔮 Version 0.3 (Future)
- [ ] **Terminal UI (TUI)** with `ratatui`
  - [ ] Interactive track selection (arrow keys)
  - [ ] Progress bar + metadata display
  - [ ] Keyboard shortcuts (Space = play/pause, etc.)
- [ ] Playlist support (queue multiple cassettes)
- [ ] Metadata editing (update tags without re-encoding)

### 🌐 Version 1.0 (Long-term)
- [ ] **Desktop GUI** (Tauri + Web UI)
  - [ ] Drag-and-drop cassette loading
  - [ ] Cover art display
  - [ ] Skeuomorphic cassette player UI
- [ ] Cassette library (recently played tapes)
- [ ] Export cassettes to ZIP (image + audio files)
- [ ] Cross-platform builds (Windows, macOS, Linux)

### 💭 Dream Features (v2.0+)
- [ ] Web-based