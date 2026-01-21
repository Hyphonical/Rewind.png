// ══════════════════════════════════════════════════════════════════════════════
// REWIND.PNG WEB PLAYER
// ══════════════════════════════════════════════════════════════════════════════
//
// Interactive web interface for the Rewind.png cassette player.
// Handles playback controls, volume, and visual states.

// ══════════════════════════════════════════════════════════════════════════════
// DOM ELEMENTS
// ══════════════════════════════════════════════════════════════════════════════

// Player controls
const playBtn = document.getElementById('btn-play');
const stopBtn = document.getElementById('btn-stop');
const fwdBtn = document.getElementById('btn-fwd');
const rewindBtn = document.getElementById('btn-rewind');
const prevBtn = document.getElementById('btn-prev');
const nextBtn = document.getElementById('btn-next');

// Visual elements
const reels = document.querySelectorAll('.reel');
const statusLed = document.getElementById('status-led');
const playIcon = playBtn.querySelector('i');

// Volume control
const volThumb = document.querySelector('.vol-thumb');
const volTrack = document.querySelector('.vol-track');

// Record panel elements
const fileDropZone = document.getElementById('file-drop-zone');
const fileInput = document.getElementById('file-input');
const fileList = document.getElementById('file-list');
const coverDropZone = document.getElementById('cover-drop-zone');
const coverInput = document.getElementById('cover-input');
const coverPreview = document.getElementById('cover-preview');
const recordBtn = document.getElementById('record-btn');
const fileCount = document.getElementById('file-count');

// ══════════════════════════════════════════════════════════════════════════════
// STATE
// ══════════════════════════════════════════════════════════════════════════════

let isPlaying = false;
let uploadedFiles = [];
let coverImage = null;

// ══════════════════════════════════════════════════════════════════════════════
// PLAYBACK CONTROLS
// ══════════════════════════════════════════════════════════════════════════════

playBtn.addEventListener('click', () => {

	if (isPlaying) {
		// Pause Logic
		isPlaying = false;
		playBtn.classList.remove('is-active');
		playIcon.classList.remove('fa-pause');
		playIcon.classList.add('fa-play');

		// Stop Animation
		reels.forEach(r => {
			r.classList.remove('spinning');
			r.classList.remove('fast-forward');
			r.classList.remove('rewind');
		});
		statusLed.classList.remove('on');
	} else {
		// Play Logic
		isPlaying = true;
		playBtn.classList.add('is-active');
		playIcon.classList.remove('fa-play');
		playIcon.classList.add('fa-pause');

		// Start Animation
		reels.forEach(r => {
			r.classList.add('spinning');
			r.classList.remove('fast-forward');
			r.classList.remove('rewind');
		});
		statusLed.classList.add('on');

		// Reset other buttons
		stopBtn.classList.remove('is-active');
	}
});

stopBtn.addEventListener('click', () => {
	isPlaying = false;

	// Visual resets
	playBtn.classList.remove('is-active');
	stopBtn.classList.add('is-active');
	setTimeout(() => stopBtn.classList.remove('is-active'), 200);

	playIcon.classList.remove('fa-pause');
	playIcon.classList.add('fa-play');

	reels.forEach(r => {
		r.classList.remove('spinning');
		r.classList.remove('fast-forward');
		r.classList.remove('rewind');
	});
	statusLed.classList.remove('on');
});

// Fast Forward Button
fwdBtn.addEventListener('mousedown', () => {
	if (isPlaying) {
		reels.forEach(r => {
			r.classList.remove('rewind');
			r.classList.add('fast-forward');
		});
		fwdBtn.classList.add('is-active');
	}
});

fwdBtn.addEventListener('mouseup', () => {
	reels.forEach(r => r.classList.remove('fast-forward'));
	fwdBtn.classList.remove('is-active');
});

fwdBtn.addEventListener('mouseleave', () => {
	reels.forEach(r => r.classList.remove('fast-forward'));
	fwdBtn.classList.remove('is-active');
});

// Rewind Button
rewindBtn.addEventListener('mousedown', () => {
	if (isPlaying) {
		reels.forEach(r => {
			r.classList.remove('fast-forward');
			r.classList.add('rewind');
		});
		rewindBtn.classList.add('is-active');
	}
});

rewindBtn.addEventListener('mouseup', () => {
	reels.forEach(r => r.classList.remove('rewind'));
	rewindBtn.classList.remove('is-active');
});

rewindBtn.addEventListener('mouseleave', () => {
	reels.forEach(r => r.classList.remove('rewind'));
	rewindBtn.classList.remove('is-active');
});

// Previous/Next buttons
prevBtn.addEventListener('click', () => {
});

nextBtn.addEventListener('click', () => {
});

// ══════════════════════════════════════════════════════════════════════════════
// VOLUME CONTROL
// ══════════════════════════════════════════════════════════════════════════════

volThumb.addEventListener('mousedown', () => {
	document.addEventListener('mousemove', updateVolume);
	document.addEventListener('mouseup', () => {
		document.removeEventListener('mousemove', updateVolume);
	}, { once: true });
});

function updateVolume(e) {
	const rect = volTrack.getBoundingClientRect();
	let x = e.clientX - rect.left;
	x = Math.max(0, Math.min(x, rect.width));
	const percent = (x / rect.width) * 100;
	volThumb.style.left = percent + '%';
}

// ══════════════════════════════════════════════════════════════════════════════
// RECORD PANEL - FILE UPLOAD
// ══════════════════════════════════════════════════════════════════════════════

// Click to open file picker
fileDropZone.addEventListener('click', () => {
	playClickSound();
	fileInput.click();
});

// Drag and drop handlers
fileDropZone.addEventListener('dragover', (e) => {
	e.preventDefault();
	fileDropZone.classList.add('dragover');
});

fileDropZone.addEventListener('dragleave', () => {
	fileDropZone.classList.remove('dragover');
});

fileDropZone.addEventListener('drop', (e) => {
	e.preventDefault();
	fileDropZone.classList.remove('dragover');
	handleFiles(e.dataTransfer.files);
});

// File input change
fileInput.addEventListener('change', (e) => {
	handleFiles(e.target.files);
});

/**
 * Handle uploaded audio files
 * @param {FileList} files - The uploaded files
 */
function handleFiles(files) {
	const validTypes = ['audio/mpeg', 'audio/mp3', 'audio/flac', 'audio/ogg', 'audio/wav', 'audio/x-flac'];

	for (const file of files) {
		// Check if file type is valid (or trust extension)
		const ext = file.name.split('.').pop().toLowerCase();
		const validExts = ['mp3', 'flac', 'ogg', 'wav'];

		if (validTypes.includes(file.type) || validExts.includes(ext)) {
			uploadedFiles.push(file);
		}
	}

	updateFileList();
}

/**
 * Update the file list display
 */
function updateFileList() {
	fileList.innerHTML = '';

	uploadedFiles.forEach((file, index) => {
		const fileItem = document.createElement('div');
		fileItem.className = 'file-item';
		fileItem.innerHTML = `
			<i class="fa-solid fa-music"></i>
			<span class="file-name">${file.name}</span>
			<span class="file-size">${formatFileSize(file.size)}</span>
			<button class="remove-file" data-index="${index}" aria-label="Remove file">
				<i class="fa-solid fa-xmark"></i>
			</button>
		`;
		fileList.appendChild(fileItem);
	});

	// Update footer count
	fileCount.textContent = `${uploadedFiles.length} FILE${uploadedFiles.length !== 1 ? 'S' : ''} READY`;

	// Add remove handlers
	document.querySelectorAll('.remove-file').forEach(btn => {
		btn.addEventListener('click', (e) => {
			e.stopPropagation();
			const index = parseInt(btn.dataset.index);
			uploadedFiles.splice(index, 1);
			updateFileList();
		});
	});
}

/**
 * Format file size to human readable string
 * @param {number} bytes - File size in bytes
 * @returns {string} Formatted size string
 */
function formatFileSize(bytes) {
	if (bytes < 1024) return bytes + ' B';
	if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
	return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

// ══════════════════════════════════════════════════════════════════════════════
// RECORD PANEL - COVER ART
// ══════════════════════════════════════════════════════════════════════════════

coverDropZone.addEventListener('click', () => {
	coverInput.click();
});

coverDropZone.addEventListener('dragover', (e) => {
	e.preventDefault();
	coverDropZone.style.borderColor = 'var(--tape-accent-blue)';
});

coverDropZone.addEventListener('dragleave', () => {
	coverDropZone.style.borderColor = '#444';
});

coverDropZone.addEventListener('drop', (e) => {
	e.preventDefault();
	coverDropZone.style.borderColor = '#444';
	if (e.dataTransfer.files.length > 0) {
		handleCoverImage(e.dataTransfer.files[0]);
	}
});

coverInput.addEventListener('change', (e) => {
	if (e.target.files.length > 0) {
		handleCoverImage(e.target.files[0]);
	}
});

/**
 * Handle uploaded cover image
 * @param {File} file - The uploaded image file
 */
function handleCoverImage(file) {
	if (!file.type.startsWith('image/')) {
		console.warn('Invalid file type for cover image');
		return;
	}

	coverImage = file;

	// Show preview
	const reader = new FileReader();
	reader.onload = (e) => {
		coverPreview.innerHTML = `<img src="${e.target.result}" alt="Cover art preview">`;
	};
	reader.readAsDataURL(file);
}

// ══════════════════════════════════════════════════════════════════════════════
// RECORD BUTTON
// ══════════════════════════════════════════════════════════════════════════════

recordBtn.addEventListener('click', () => {

	// Visual feedback only - actual recording would happen here
	if (uploadedFiles.length > 0 && coverImage) {
		console.log('Recording cassette with', uploadedFiles.length, 'tracks');
		console.log('Cover image:', coverImage.name);

		// Could show a recording animation/state here
		statusLed.classList.add('recording');
		setTimeout(() => {
			statusLed.classList.remove('recording');
		}, 2000);
	} else {
		console.log('Need files and cover image to record');
	}
});

// ══════════════════════════════════════════════════════════════════════════════
// TRACK SELECTION
// ══════════════════════════════════════════════════════════════════════════════

document.querySelectorAll('.track-item').forEach(item => {
	item.addEventListener('click', () => {
		// Remove active from all
		document.querySelectorAll('.track-item').forEach(i => i.classList.remove('active'));
		// Add active to clicked
		item.classList.add('active');
	});
});
