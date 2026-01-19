<!-- filepath: c:\Users\joris\Desktop\Rewind.png\README.md -->
# 🎵 Rewind.png

> **Digital cassette tapes disguised as PNG images**

Rewind.png is an experimental media format that embeds lossless audio (FLAC/MP3/OGG/WAV) inside PNG images without corrupting the visual data. Think of it as a modern cassette tape; The PNG is the box art, and the embedded audio is the music inside.

![Version](https://img.shields.io/badge/version-0.5.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Rust](https://img.shields.io/badge/rust-1.70+-orange)

---

## 🎬 Demo

<details>

<summary><strong>📼 Click to view the demo cassette</strong> (27.5 MB)</summary>

![Demo Cassette](demo/Cassette.png)

</details>

**Try it:** `rewind tui demo/Cassette.png`

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

### System Requirements
- **OS**: Windows 10+, macOS 10.15+, or Linux (any modern distro)
- **Terminal**: Unicode support recommended for TUI (Windows Terminal, iTerm2, etc.)
- **Audio**: Working audio output device

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

### 4. Interactive TUI Player (NEW in v0.5!)
Open the full-featured skeuomorphic cassette player:

```bash
rewind tui mixtape.png
```

**Features:**
- 🎨 **Vintage cassette design** with animated progress bar and spinning reels
- 🖱️ **Mouse support** - click buttons directly or select tracks
- 🔊 **Volume control** with visual slider (0-100%)
- 📜 **Dynamic playlist** that auto-sizes based on track count
- ⏯️ **Full playback controls** with visual feedback

**Keyboard Controls:**
| Key | Action |
|-----|--------|
| ↑/↓ or j/k | Navigate tracks |
| Enter | Play selected track |
| Space | Pause/Resume |
| ←/→ or p/n | Previous/Next track |
| +/- | Volume up/down |
| S | Stop playback |
| Q or Esc | Quit |

**Mouse Controls:**
- Click on any button (⏮ ▶ ⏸ ■ ⏭) to control playback
- Click on a track in the playlist to play it
- Click volume buttons to adjust audio level

> **Note:** Your terminal must support Unicode box-drawing characters and mouse input for the best experience.

---

## 🗺️ Roadmap

### ✅ Version 0.1 - Foundation (Completed)
- [x] Core polyglot PNG/audio embedding
- [x] Record audio tracks into PNG images
- [x] Inspect cassette metadata & verify integrity
- [x] Basic CLI playback

### ✅ Version 0.2 - CLI Refinement (Completed)
- [x] Structured CLI with `clap`
  - [x] `rewind record <image> <audio...> -o <output>`
  - [x] `rewind inspect <cassette>`
  - [x] `rewind play <cassette> [--track N]`
- [x] Sequential playback (auto-advance tracks)
- [x] Improved error handling and user feedback

### ✅ Version 0.3 - Basic TUI (Completed)
- [x] Terminal UI with `ratatui`
- [x] Interactive track selection
- [x] Progress bar + metadata display
- [x] Keyboard navigation

### ✅ Version 0.4 - Enhanced Playback (Completed)
- [x] Improved audio handling with `rodio`
- [x] Track metadata extraction (artist, title, duration)
- [x] CRC32 integrity verification
- [x] Pause/resume functionality

### ✅ Version 0.5 - Skeuomorphic TUI (Current)
- [x] Complete UI redesign with vintage cassette aesthetics
- [x] Mouse support (clickable buttons)
- [x] Volume control with visual slider
- [x] Dynamic playlist sizing
- [x] Auto-play next track

---

### 🚧 Version 0.6 - Polish & Distribution (Planned)
- [x] Pre-built binaries for Windows, macOS, Linux
- [ ] Installer/package manager support
- [ ] Configuration file support (default volume, theme)
- [ ] Error recovery (handle corrupted tracks gracefully)

### 🎯 Version 1.0 - Desktop GUI (Future)
- [ ] Native desktop app with Tauri
- [ ] Drag-and-drop cassette loading
- [ ] Visual cover art display
- [ ] Cassette library (recently played)

### 🌟 Version 2.0+ - Advanced Features (Dreams)
- [ ] Web-based player (WASM)
- [ ] Multi-cassette mixtapes

---

## 📜 License & Attribution

### Code
This project is licensed under the **MIT License**. See [LICENSE](LICENSE) for details.

### Demo Content

**Demo Cassette Image (`demo/Cassette.png`):**
- Cover art © 2025 Hyphonical. Licensed under [Creative Commons Attribution-NonCommercial-NoDerivatives 4.0 International (CC BY-NC-ND 4.0)](https://creativecommons.org/licenses/by-nc-nd/4.0/).
- **You may:** Share and redistribute the image with proper attribution
- **You may not:** Modify, remix, or create derivative works, or use it for commercial purposes.
- The Sony® trademark shown on the demo cassette is used for aesthetic purposes only. Sony is a registered trademark of Sony Corporation. This project is not affiliated with, endorsed by, or sponsored by Sony Corporation.

**Demo Audio Tracks:**  
All tracks by **Kevin MacLeod** ([incompetech.com](https://incompetech.com)):
- "Aquarium"
- "Sugar Plum Breakdown"
- "Hall of the Mountain King"

Licensed under [Creative Commons: By Attribution 4.0 License](http://creativecommons.org/licenses/by/4.0/)

---

## 📝 Notes

A great part of this project was created using various AI models, including Claude Sonnet & Opus, Gemini 3 Pro & Flash, and Grok Code Fast 1. Without these tools I would not have been able to create Rewind.png in this timeframe.

---

**Made with 💖**