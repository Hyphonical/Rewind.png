// ══════════════════════════════════════════════════════════════════════════════
// CONSTANTS MODULE
// ══════════════════════════════════════════════════════════════════════════════
//
// Defines application-wide constants used throughout the codebase.
// - IEND_CHUNK: PNG end-of-file marker (where we append audio data)
// - BUFFER_SIZE: Optimal buffer size for file I/O operations

pub const IEND_CHUNK: [u8; 12] = [
	0x00, 0x00, 0x00, 0x00,
	0x49, 0x45, 0x4E, 0x44,
	0xAE, 0x42, 0x60, 0x82
];

pub const BUFFER_SIZE: usize = 16384;