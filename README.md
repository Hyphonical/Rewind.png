<!-- filepath: c:\Users\joris\Desktop\Rewind.png\README.md -->
# 🎵 Rewind.png

> **Digital cassette tapes disguised as PNG images**

Rewind.png is an experimental media format that embeds lossless audio (FLAC/MP3/OGG/WAV) inside PNG images without corrupting the visual data. Think of it as a modern cassette tape; The PNG is the box art, and the embedded audio is the music inside.

![Version](https://img.shields.io/badge/version-0.3.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Rust](https://img.shields.io/badge/rust-1.70+-orange)

---

## 🎬 Demo

<details>

<summary><strong>📼 Click to view demo cassette</strong> (27.5 MB)</summary>

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

### 4. Interactive TUI Player (NEW in v0.3!)
Open the full-featured terminal player:

```bash
rewind tui mixtape.png
```

**Controls:**
| Key | Action |
|-----|--------|
| ↑/↓ or j/k | Navigate tracks |
| Enter | Play selected track |
| Space | Pause/Resume |
| ←/→ or p/n | Previous/Next track |
| S | Stop playback |
| Q or Esc | Quit |

---

## 🗺️ Roadmap

### ✅ Version 0.1 (Completed)
- [x] Record audio into PNG images
- [x] Inspect cassette metadata & verify integrity
- [x] Random track playback (CLI-ish)

### 🚧 Version 0.2 (Completed)
- [x] CLI argument parsing (`clap`)
  - [x] `rewind record <image> <audio...> -o <output>`
  - [x] `rewind inspect <cassette>`
  - [x] `rewind play <cassette> [--track N]`
  - [x] `rewind extract <cassette> <track_number> -o output.flac`
- [x] Sequential playback (play all tracks)
- [x] Track extraction (`rewind extract <cassette> <track_number> -o output.flac`)
- [x] Better error messages (user-friendly)

### 🔮 Version 0.3 (Completed)
- [x] **Terminal UI (TUI)** with `ratatui`
  - [x] Interactive track selection (arrow keys)
  - [x] Progress bar + metadata display
  - [x] Keyboard shortcuts (Space = play/pause, etc.)

### 🌐 Version 1.0
- [ ] **Desktop GUI** (Tauri + Web UI)
  - [ ] Drag-and-drop cassette loading
  - [ ] Cover art display
  - [ ] Skeuomorphic cassette player UI
- [ ] Cassette library (recently played tapes)
- [ ] Export cassettes to ZIP (image + audio files)
- [ ] Cross-platform builds (Windows, macOS, Linux)

### 💭 Dream Features (v2.0+)
- [ ] Web-based player (WASM compilation)
- [ ] Cassette sharing platform
- [ ] Tape degradation effects (authentic lo-fi simulation)

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