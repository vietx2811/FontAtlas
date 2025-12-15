// FontAtlas Frontend JavaScript

let allFonts = [];
let currentDirectory = '../fonts/samples';

// LocalStorage keys
const STORAGE_KEY_DIRECTORIES = 'fontatlas_directories';
const STORAGE_KEY_CURRENT = 'fontatlas_current_directory';

// Load fonts on page load
document.addEventListener('DOMContentLoaded', () => {
    loadSavedDirectories();
    loadDirectories();
    loadFonts();
});

function loadSavedDirectories() {
    // Load from localStorage
    const saved = localStorage.getItem(STORAGE_KEY_DIRECTORIES);
    const savedCurrent = localStorage.getItem(STORAGE_KEY_CURRENT);

    if (saved) {
        const directories = JSON.parse(saved);
        const select = document.getElementById('font-directory');

        // Add saved directories to dropdown
        directories.forEach(dir => {
            if (!Array.from(select.options).find(opt => opt.value === dir)) {
                const option = document.createElement('option');
                option.value = dir;
                option.textContent = dir;
                select.appendChild(option);
            }
        });
    }

    // Restore last selected directory
    if (savedCurrent) {
        currentDirectory = savedCurrent;
        document.getElementById('font-directory').value = savedCurrent;
    }
}

function saveDirectoryToHistory(path) {
    // Save to localStorage
    const saved = localStorage.getItem(STORAGE_KEY_DIRECTORIES);
    let directories = saved ? JSON.parse(saved) : [];

    // Add if not already in list
    if (!directories.includes(path)) {
        directories.push(path);
        localStorage.setItem(STORAGE_KEY_DIRECTORIES, JSON.stringify(directories));
    }

    // Save current directory
    localStorage.setItem(STORAGE_KEY_CURRENT, path);
}

function addCustomPath() {
    const input = document.getElementById('custom-path');
    const path = input.value.trim();

    if (!path) {
        alert('Please enter a directory path');
        return;
    }

    // Add to dropdown
    const select = document.getElementById('font-directory');
    const existingOption = Array.from(select.options).find(opt => opt.value === path);

    if (!existingOption) {
        const option = document.createElement('option');
        option.value = path;
        option.textContent = path;
        select.appendChild(option);
    }

    select.value = path;
    currentDirectory = path;

    // Save to history
    saveDirectoryToHistory(path);

    // Clear input
    input.value = '';

    // Load fonts
    loadFonts();
}

async function loadDirectories() {
    try {
        const response = await fetch('/api/fonts/directories');
        const data = await response.json();

        const select = document.getElementById('font-directory');

        // Get saved directories from localStorage
        const saved = localStorage.getItem(STORAGE_KEY_DIRECTORIES);
        const savedDirs = saved ? JSON.parse(saved) : [];

        // Merge saved + API directories (remove duplicates)
        const allDirs = new Set([...savedDirs, ...data.directories.map(d => d.path)]);

        // Clear and rebuild dropdown
        select.innerHTML = '';

        // Add all unique directories
        allDirs.forEach(dirPath => {
            // Find full directory info from API if available
            const dirInfo = data.directories.find(d => d.path === dirPath);

            const option = document.createElement('option');
            option.value = dirPath;

            if (dirInfo) {
                option.textContent = `${dirPath} (${dirInfo.profile_files} profiles, ${dirInfo.ttf_files} fonts)`;
            } else {
                option.textContent = dirPath;
            }

            if (dirPath === currentDirectory) {
                option.selected = true;
            }
            select.appendChild(option);
        });
    } catch (error) {
        console.error('Error loading directories:', error);
    }
}

function loadFontsFromDirectory() {
    currentDirectory = document.getElementById('font-directory').value;
    saveDirectoryToHistory(currentDirectory);
    loadFonts();
}

function refreshDirectories() {
    loadDirectories();
    loadFonts();
}

// Font Scanning Functions
let scanTaskId = null;
let scanInterval = null;

async function scanCurrentDirectory() {
    if (!currentDirectory) {
        alert('Please select a directory first');
        return;
    }

    if (!confirm(`Scan and analyze all fonts in:\n${currentDirectory}\n\nThis will create .complete.json profiles for all .ttf files.\n\nContinue?`)) {
        return;
    }

    // Get force rescan option
    const forceRescan = document.getElementById('force-rescan').checked;

    // Show progress bar
    document.getElementById('scan-progress').style.display = 'block';
    document.getElementById('progress-status').textContent = forceRescan ? 'Starting rescan (force)...' : 'Starting scan...';
    document.getElementById('progress-percent').textContent = '0%';
    document.getElementById('progress-fill').style.width = '0%';

    try {
        // Start scan
        const response = await fetch('/api/fonts/scan', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                directory: currentDirectory,
                force: forceRescan
            })
        });

        const data = await response.json();
        scanTaskId = data.task_id;

        // Poll for progress
        scanInterval = setInterval(checkScanProgress, 500);

    } catch (error) {
        console.error('Error starting scan:', error);
        alert('Failed to start scan');
        document.getElementById('scan-progress').style.display = 'none';
    }
}

async function checkScanProgress() {
    if (!scanTaskId) return;

    try {
        const response = await fetch(`/api/fonts/scan/${scanTaskId}/progress`);
        const progress = await response.json();

        // Update UI
        const percent = progress.total > 0
            ? Math.round((progress.current / progress.total) * 100)
            : 0;

        document.getElementById('progress-percent').textContent = `${percent}%`;
        document.getElementById('progress-fill').style.width = `${percent}%`;
        document.getElementById('progress-file').textContent = progress.current_file
            ? `Processing: ${progress.current_file}`
            : '';

        if (progress.status === 'complete') {
            document.getElementById('progress-status').textContent = '✓ Scan complete!';
            clearInterval(scanInterval);
            scanTaskId = null;

            // Reload fonts after 1 second
            setTimeout(() => {
                document.getElementById('scan-progress').style.display = 'none';
                loadFonts();
            }, 1500);

        } else if (progress.status === 'error') {
            document.getElementById('progress-status').textContent = '✗ Scan failed';
            document.getElementById('progress-status').style.color = '#e74c3c';
            clearInterval(scanInterval);
            scanTaskId = null;

            setTimeout(() => {
                document.getElementById('scan-progress').style.display = 'none';
            }, 3000);
        } else {
            document.getElementById('progress-status').textContent =
                `Scanning... (${progress.current}/${progress.total})`;
        }

    } catch (error) {
        console.error('Error checking progress:', error);
    }
}

// Qdrant Indexing Functions
let indexTaskId = null;
let indexInterval = null;

async function indexToQdrant() {
    if (!currentDirectory) {
        alert('Please select a directory first');
        return;
    }

    if (!confirm(`Index all font profiles to Qdrant?\n\nDirectory: ${currentDirectory}\n\nThis will enable font pairing search.\n\nContinue?`)) {
        return;
    }

    // Show progress bar
    const progressDiv = document.getElementById('scan-progress');
    progressDiv.style.display = 'block';
    document.getElementById('progress-status').textContent = 'Starting indexing...';
    document.getElementById('progress-percent').textContent = '0%';
    document.getElementById('progress-fill').style.width = '0%';

    try {
        // Start indexing
        const response = await fetch('/api/fonts/index', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ directory: currentDirectory })
        });

        const data = await response.json();
        indexTaskId = data.task_id;

        // Poll for progress
        indexInterval = setInterval(checkIndexProgress, 500);

    } catch (error) {
        console.error('Error starting indexing:', error);
        alert('Failed to start indexing');
        progressDiv.style.display = 'none';
    }
}

async function checkIndexProgress() {
    if (!indexTaskId) return;

    try {
        const response = await fetch(`/api/fonts/index/${indexTaskId}/progress`);
        const progress = await response.json();

        // Update UI
        const percent = progress.total > 0
            ? Math.round((progress.current / progress.total) * 100)
            : 0;

        document.getElementById('progress-percent').textContent = `${percent}%`;
        document.getElementById('progress-fill').style.width = `${percent}%`;
        document.getElementById('progress-file').textContent = progress.current_file
            ? `Indexing: ${progress.current_file}`
            : '';

        if (progress.status === 'complete') {
            document.getElementById('progress-status').textContent = '✓ Indexing complete!';
            clearInterval(indexInterval);
            indexTaskId = null;

            setTimeout(() => {
                document.getElementById('scan-progress').style.display = 'none';
            }, 1500);

        } else if (progress.status === 'error') {
            document.getElementById('progress-status').textContent = '✗ Indexing failed';
            document.getElementById('progress-status').style.color = '#e74c3c';
            clearInterval(indexInterval);
            indexTaskId = null;

            setTimeout(() => {
                document.getElementById('scan-progress').style.display = 'none';
            }, 3000);
        } else {
            document.getElementById('progress-status').textContent =
                `Indexing... (${progress.current}/${progress.total})`;
        }

    } catch (error) {
        console.error('Error checking indexing progress:', error);
    }
}

async function loadFonts() {
    try {
        const response = await fetch(`/api/fonts/?directory=${encodeURIComponent(currentDirectory)}`);
        const data = await response.json();
        allFonts = data.fonts;

        displayFonts(allFonts);
        populateFontSelects(allFonts);
    } catch (error) {
        console.error('Error loading fonts:', error);
        document.getElementById('font-list').innerHTML = '<p class="error">Failed to load fonts</p>';
    }
}

function displayFonts(fonts) {
    const container = document.getElementById('font-list');

    if (fonts.length === 0) {
        container.innerHTML = '<p>No fonts found</p>';
        return;
    }

    // Format metrics with N/A for zero or null values
    const formatMetric = (value, decimals = 3) => {
        return (value === 0 || value === null) ? 'N/A' : value.toFixed(decimals);
    };

    // Load fonts dynamically
    fonts.forEach(font => {
        const fontFamily = `font-${font.id}`;
        const fontUrl = `/api/fonts/font-file/${encodeURIComponent(font.id)}?directory=${encodeURIComponent(currentDirectory)}`;

        // Create @font-face rule
        const style = document.createElement('style');
        style.textContent = `
            @font-face {
                font-family: '${fontFamily}';
                src: url('${fontUrl}') format('truetype');
            }
        `;
        document.head.appendChild(style);
    });

    container.innerHTML = fonts.map(font => `
        <div class="font-card">
            <div class="font-preview" style="font-family: 'font-${font.id}', serif;">
                Aa Bb Cc
            </div>
            <h3>${font.name}</h3>
            <p class="category">${font.category}</p>
            <div class="metrics">
                <div class="metric">
                    <span class="label">X-Height:</span>
                <span class="value">${formatMetric(font.x_height)}</span>
                </div>
                <div class="metric">
                    <span class="label">Width Var:</span>
                    <span class="value">${formatMetric(font.width_variance)}</span>
                </div>
                <div class="metric">
                    <span class="label">Stroke Contrast:</span>
                    <span class="value">${formatMetric(font.stroke_contrast, 2)}</span>
                </div>
                <div class="metric">
                    <span class="label">Aperture:</span>
                    <span class="value">${formatMetric(font.aperture, 2)}</span>
                </div>
            </div>
        </div>
    `).join('');
}

function populateFontSelects(fonts) {
    const selects = [
        document.getElementById('query-font'),
        document.getElementById('anatomy-font')
    ];

    selects.forEach(select => {
        select.innerHTML = '<option value="">Select a font...</option>';
        fonts.forEach(font => {
            const option = document.createElement('option');
            option.value = font.id;
            option.textContent = font.name;
            select.appendChild(option);
        });
    });
}

async function findPairs() {
    const queryFont = document.getElementById('query-font').value;
    const resultsDiv = document.getElementById('pairing-results');

    if (!queryFont) {
        resultsDiv.innerHTML = '<p class="error">Please select a font</p>';
        return;
    }

    resultsDiv.innerHTML = '<p>Finding pairs...</p>';

    try {
        const response = await fetch('/api/pairs/find?directory=' + encodeURIComponent(currentDirectory), {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                font_id: queryFont,
                limit: 3,
                explain: false
            })
        });

        const data = await response.json();

        if (data.pairs.length === 0) {
            resultsDiv.innerHTML = '<p>No pairs found</p>';
            return;
        }

        resultsDiv.innerHTML = `
            <h3>Query Font: ${data.query_font}</h3>
            <div class="pairing-list">
                ${data.pairs.map((pair, i) => `
                    <div class="pairing-result">
                        <div class="rank">#${i + 1}</div>
                        <h4>${pair.font_name}</h4>
                        <div class="score-badge">${(pair.score * 100).toFixed(0)}%</div>
                        <div class="component-scores">
                            <div class="score-item">
                                <span>X-Height:</span> <strong>${(pair.components.x_height_score * 100).toFixed(0)}%</strong>
                            </div>
                            <div class="score-item">
                                <span>Width:</span> <strong>${(pair.components.width_variance_score * 100).toFixed(0)}%</strong>
                            </div>
                            <div class="score-item">
                                <span>Stroke:</span> <strong>${(pair.components.stroke_contrast_score * 100).toFixed(0)}%</strong>
                            </div>
                            <div class="score-item">
                                <span>Aperture:</span> <strong>${(pair.components.aperture_score * 100).toFixed(0)}%</strong>
                            </div>
                        </div>
                    </div>
                `).join('')}
            </div>
        `;
    } catch (error) {
        console.error('Error finding pairs:', error);
        resultsDiv.innerHTML = '<p class="error">Failed to find pairs</p>';
    }
}

async function showAnatomy() {
    const fontId = document.getElementById('anatomy-font').value;
    const displayDiv = document.getElementById('anatomy-display');

    if (!fontId) {
        displayDiv.innerHTML = '<p class="error">Please select a font</p>';
        return;
    }

    displayDiv.innerHTML = '<p>Loading anatomy data...</p>';

    // Load font for preview
    const fontFamily = `font-${fontId}`;
    const fontUrl = `/api/fonts/font-file/${encodeURIComponent(fontId)}?directory=${encodeURIComponent(currentDirectory)}`;
    const style = document.createElement('style');
    style.textContent = `
        @font-face {
            font-family: '${fontFamily}';
            src: url('${fontUrl}') format('truetype');
        }
    `;
    document.head.appendChild(style);

    try {
        // Load stroke contrast
        const strokeResponse = await fetch(`/api/anatomy/stroke-contrast/${fontId}?directory=${encodeURIComponent(currentDirectory)}`);
        const strokeData = await strokeResponse.json();

        // Load aperture
        const apertureResponse = await fetch(`/api/anatomy/aperture/${fontId}?directory=${encodeURIComponent(currentDirectory)}`);
        const apertureData = await apertureResponse.json();

        displayDiv.innerHTML = `
            <div class="font-specimen" style="font-family: '${fontFamily}', serif;">
                <div style="font-size: 64px; margin-bottom: 20px; color: #2c3e50;">
                    The quick brown fox jumps
                </div>
                <div style="font-size: 24px; color: #7f8c8d; margin-bottom: 32px;">
                    ABCDEFGHIJKLMNOPQRSTUVWXYZ<br>
                    abcdefghijklmnopqrstuvwxyz<br>
                    0123456789 !@#$%^&*()
                </div>
            </div>
            
            <h3>${strokeData.font}</h3>
            
            <div class="anatomy-section">
                <h4>Stroke Contrast Analysis</h4>
                <p class="classification">Classification: <strong>${strokeData.classification.toUpperCase()}</strong></p>
                <p>Mean Contrast Ratio: <strong>${strokeData.mean_contrast.toFixed(2)}</strong></p>
                <div class="glyph-grid">
                    ${Object.entries(strokeData.by_glyph).map(([glyph, data]) => `
                        <div class="glyph-detail">
                            <div class="glyph-char">${glyph}</div>
                            <div class="glyph-info">Ratio: ${data.contrast_ratio.toFixed(2)}</div>
                        </div>
                    `).join('')}
                </div>
            </div>
            
            <div class="anatomy-section">
                <h4>Aperture Openness</h4>
                <p class="classification">Classification: <strong>${apertureData.classification.toUpperCase()}</strong></p>
                <p>Mean Aperture: <strong>${apertureData.mean_aperture.toFixed(2)}</strong></p>
                <div class="glyph-grid">
                    ${Object.entries(apertureData.by_glyph).map(([glyph, data]) => `
                        <div class="glyph-detail">
                            <div class="glyph-char">${glyph}</div>
                            <div class="glyph-info">${data.score.toFixed(2)} [${data.detection_method}]</div>
                        </div>
                    `).join('')}
                </div>
            </div>
        `;
    } catch (error) {
        console.error('Error loading anatomy:', error);
        displayDiv.innerHTML = '<p class="error">Failed to load anatomy data</p>';
    }
}
