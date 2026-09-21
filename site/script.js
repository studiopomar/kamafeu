/* ===================================================================
   KAMAFEU STUDIO - WORKSTATION PORTAL JAVASCRIPT
   WebAudio Formant Synthesizer, Piano Roll, oto.ini Calibrator & UI
   =================================================================== */

document.addEventListener('DOMContentLoaded', () => {
  initOsDetection();
  initVocalSynthKeyboard();
  initOtoCalibrator();
  initThemeGallery();
  initCompilationTabs();
  initMobileNav();
});

/* -------------------------------------------------------------------
   1. OS DETECTION FOR HERO DOWNLOAD
   ------------------------------------------------------------------- */
function initOsDetection() {
  const downloadText = document.getElementById('osDownloadText');
  const heroBtn = document.getElementById('heroDownloadBtn');
  if (!downloadText || !heroBtn) return;

  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes('mac')) {
    downloadText.textContent = 'Baixar para macOS (.dmg)';
  } else if (ua.includes('win')) {
    downloadText.textContent = 'Baixar para Windows (.exe)';
  } else if (ua.includes('android')) {
    downloadText.textContent = 'Baixar APK (.apk)';
  } else if (ua.includes('linux')) {
    downloadText.textContent = 'Baixar para Linux (.AppImage)';
  } else {
    downloadText.textContent = 'Ver Downloads Disponíveis';
  }
}

/* -------------------------------------------------------------------
   2. WEBAUDIO VOCAL SYNTH & PIANO ROLL KEYBOARD
   ------------------------------------------------------------------- */
const VOWEL_FORMANTS = {
  a: { f1: 800, f2: 1200, f3: 2500, q1: 6, q2: 6, q3: 8 },
  e: { f1: 500, f2: 1800, f3: 2600, q1: 6, q2: 7, q3: 8 },
  i: { f1: 300, f2: 2200, f3: 3000, q1: 7, q2: 8, q3: 9 },
  o: { f1: 500, f2: 850,  f3: 2400, q1: 6, q2: 6, q3: 7 },
  u: { f1: 320, f2: 800,  f3: 2200, q1: 7, q2: 6, q3: 7 }
};

let audioCtx = null;
let activeOsc = null;
let activeGain = null;
let activeLfo = null;
let isVibratoOn = true;
let synthAnalyser = null;
let synthAnimId = null;
let phraseTimeouts = [];

function ensureAudioContext() {
  if (!audioCtx) {
    const AudioContextClass = window.AudioContext || window.webkitAudioContext;
    audioCtx = new AudioContextClass();
  }
  if (audioCtx.state === 'suspended') {
    audioCtx.resume();
  }
  if (!synthAnalyser) {
    synthAnalyser = audioCtx.createAnalyser();
    synthAnalyser.fftSize = 512;
  }
}

function initVocalSynthKeyboard() {
  const keys = document.querySelectorAll('.piano-keyboard .key');
  const vowelSelect = document.getElementById('synthVowel');
  const pitchSlider = document.getElementById('synthPitchBend');
  const pitchVal = document.getElementById('synthPitchVal');
  const vibratoBtn = document.getElementById('synthVibratoBtn');
  const playKamafeuBtn = document.getElementById('btnPlayKamafeu');
  const playPomarBtn = document.getElementById('btnPlayPomar');
  const stopBtn = document.getElementById('btnStopSynth');
  const statusInfo = document.getElementById('synthStatusInfo');
  const canvas = document.getElementById('synthOscCanvas');

  // Pitch Bend Slider
  if (pitchSlider && pitchVal) {
    pitchSlider.addEventListener('input', (e) => {
      const cents = parseInt(e.target.value, 10);
      pitchVal.textContent = `${cents > 0 ? '+' : ''}${cents} cents`;
      if (activeOsc && audioCtx) {
        const baseFreq = parseFloat(activeOsc.datasetBaseFreq || 261.63);
        const targetFreq = baseFreq * Math.pow(2, cents / 1200);
        activeOsc.frequency.setTargetAtTime(targetFreq, audioCtx.currentTime, 0.02);
      }
    });
  }

  // Vibrato Toggle
  if (vibratoBtn) {
    vibratoBtn.addEventListener('click', () => {
      isVibratoOn = !isVibratoOn;
      vibratoBtn.classList.toggle('active', isVibratoOn);
      vibratoBtn.textContent = isVibratoOn ? '6.0 Hz / 45c' : 'Desativado';
    });
  }

  // Piano Key Press Events
  keys.forEach(k => {
    const note = k.getAttribute('data-note');
    const freq = parseFloat(k.getAttribute('data-freq'));

    const pressKey = (e) => {
      e.preventDefault();
      clearActivePhrases();
      playSynthVoice(freq, note);
      k.classList.add('playing');
    };

    const releaseKey = (e) => {
      e.preventDefault();
      stopSynthVoice();
      k.classList.remove('playing');
    };

    k.addEventListener('mousedown', pressKey);
    k.addEventListener('mouseup', releaseKey);
    k.addEventListener('mouseleave', releaseKey);

    k.addEventListener('touchstart', pressKey, { passive: false });
    k.addEventListener('touchend', releaseKey, { passive: false });
  });

  // Melodies
  if (playKamafeuBtn) {
    playKamafeuBtn.addEventListener('click', () => {
      playVocalSequence([
        { note: 'C4', freq: 261.63, vowel: 'a', lyric: 'Ka', dur: 320 },
        { note: 'D4', freq: 293.66, vowel: 'a', lyric: 'ma', dur: 320 },
        { note: 'E4', freq: 329.63, vowel: 'e', lyric: 'feu', dur: 450 },
        { note: 'G4', freq: 392.00, vowel: 'u', lyric: 'Stu-', dur: 300 },
        { note: 'A4', freq: 440.00, vowel: 'o', lyric: 'dio', dur: 700 }
      ]);
    });
  }

  if (playPomarBtn) {
    playPomarBtn.addEventListener('click', () => {
      playVocalSequence([
        { note: 'E4', freq: 329.63, vowel: 'u', lyric: 'Stu-', dur: 280 },
        { note: 'G4', freq: 392.00, vowel: 'i', lyric: 'dio', dur: 280 },
        { note: 'C5', freq: 523.25, vowel: 'o', lyric: 'Po-', dur: 380 },
        { note: 'G4', freq: 392.00, vowel: 'a', lyric: 'mar', dur: 700 }
      ]);
    });
  }

  if (stopBtn) {
    stopBtn.addEventListener('click', () => {
      clearActivePhrases();
      stopSynthVoice();
      if (statusInfo) statusInfo.textContent = 'Parado';
    });
  }

  if (canvas) {
    setupSynthOscilloscope(canvas);
  }
}

function playSynthVoice(baseFreq, noteName) {
  ensureAudioContext();
  stopSynthVoice();

  const vowelSelect = document.getElementById('synthVowel');
  const vowel = vowelSelect ? vowelSelect.value : 'a';
  const formants = VOWEL_FORMANTS[vowel] || VOWEL_FORMANTS.a;

  const pitchSlider = document.getElementById('synthPitchBend');
  const cents = pitchSlider ? parseInt(pitchSlider.value, 10) : 0;
  const targetFreq = baseFreq * Math.pow(2, cents / 1200);

  const now = audioCtx.currentTime;

  // Master Gain with human vocal envelope (attack 35ms)
  activeGain = audioCtx.createGain();
  activeGain.gain.setValueAtTime(0.0001, now);
  activeGain.gain.exponentialRampToValueAtTime(0.45, now + 0.035);

  // Glottal Sawtooth Waveform
  activeOsc = audioCtx.createOscillator();
  activeOsc.type = 'sawtooth';
  activeOsc.frequency.setValueAtTime(targetFreq, now);
  activeOsc.datasetBaseFreq = baseFreq;

  // Sub warmth
  const subOsc = audioCtx.createOscillator();
  subOsc.type = 'square';
  subOsc.frequency.setValueAtTime(targetFreq * 0.5, now);
  const subGain = audioCtx.createGain();
  subGain.gain.setValueAtTime(0.08, now);
  subOsc.connect(subGain);

  // Vibrato LFO
  if (isVibratoOn) {
    activeLfo = audioCtx.createOscillator();
    activeLfo.frequency.setValueAtTime(6.0, now);
    const lfoGain = audioCtx.createGain();
    lfoGain.gain.setValueAtTime(5.0, now);
    activeLfo.connect(lfoGain);
    lfoGain.connect(activeOsc.frequency);
    activeLfo.start(now);
    activeOsc.lfo = activeLfo;
  }

  // Formant Filter Bank (F1, F2, F3 parallel bandpasses)
  const fMix = audioCtx.createGain();
  fMix.gain.setValueAtTime(0.85, now);

  const f1 = audioCtx.createBiquadFilter();
  f1.type = 'bandpass';
  f1.frequency.setValueAtTime(formants.f1, now);
  f1.Q.setValueAtTime(formants.q1, now);

  const f2 = audioCtx.createBiquadFilter();
  f2.type = 'bandpass';
  f2.frequency.setValueAtTime(formants.f2, now);
  f2.Q.setValueAtTime(formants.q2, now);

  const f3 = audioCtx.createBiquadFilter();
  f3.type = 'bandpass';
  f3.frequency.setValueAtTime(formants.f3, now);
  f3.Q.setValueAtTime(formants.q3, now);

  activeOsc.connect(f1);
  activeOsc.connect(f2);
  activeOsc.connect(f3);
  subGain.connect(f1);

  f1.connect(fMix);
  f2.connect(fMix);
  f3.connect(fMix);

  fMix.connect(activeGain);
  activeGain.connect(synthAnalyser);
  synthAnalyser.connect(audioCtx.destination);

  activeOsc.start(now);
  subOsc.start(now);
  activeOsc.subOsc = subOsc;

  const statusInfo = document.getElementById('synthStatusInfo');
  if (statusInfo) {
    statusInfo.textContent = `Sintetizando: ${noteName} (${targetFreq.toFixed(1)} Hz) — Vogal [ ${vowel.toUpperCase()} ] — Motor VENUS`;
  }
}

function stopSynthVoice() {
  if (activeGain && audioCtx) {
    const now = audioCtx.currentTime;
    activeGain.gain.cancelScheduledValues(now);
    activeGain.gain.setValueAtTime(activeGain.gain.value, now);
    activeGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.05);

    const oldOsc = activeOsc;
    setTimeout(() => {
      try {
        if (oldOsc) {
          oldOsc.stop();
          if (oldOsc.subOsc) oldOsc.subOsc.stop();
          if (oldOsc.lfo) oldOsc.lfo.stop();
        }
      } catch (e) {}
    }, 60);

    activeOsc = null;
    activeGain = null;
  }
}

function clearActivePhrases() {
  phraseTimeouts.forEach(t => clearTimeout(t));
  phraseTimeouts = [];
  document.querySelectorAll('.piano-keyboard .key').forEach(k => k.classList.remove('playing'));
}

function playVocalSequence(notes) {
  ensureAudioContext();
  clearActivePhrases();
  stopSynthVoice();

  let delay = 0;
  notes.forEach((item, idx) => {
    const t = setTimeout(() => {
      document.querySelectorAll('.piano-keyboard .key').forEach(k => k.classList.remove('playing'));
      const keyEl = document.querySelector(`.piano-keyboard .key[data-note="${item.note}"]`);
      if (keyEl) keyEl.classList.add('playing');

      const vowelSelect = document.getElementById('synthVowel');
      if (vowelSelect) vowelSelect.value = item.vowel;

      playSynthVoice(item.freq, `${item.note} ("${item.lyric}")`);

      const stopT = setTimeout(() => {
        stopSynthVoice();
        if (keyEl) keyEl.classList.remove('playing');
        if (idx === notes.length - 1) {
          const statusInfo = document.getElementById('synthStatusInfo');
          if (statusInfo) statusInfo.textContent = 'Sequência finalizada — Pronto';
        }
      }, item.dur - 35);
      phraseTimeouts.push(stopT);
    }, delay);

    phraseTimeouts.push(t);
    delay += item.dur;
  });
}

function setupSynthOscilloscope(canvas) {
  const ctx = canvas.getContext('2d');
  const bufferLen = 256;
  const dataArray = new Uint8Array(bufferLen);

  function draw() {
    synthAnimId = requestAnimationFrame(draw);
    const w = canvas.width;
    const h = canvas.height;

    ctx.fillStyle = '#040608';
    ctx.fillRect(0, 0, w, h);

    // Subtle grid line
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.06)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();

    if (synthAnalyser && activeGain) {
      synthAnalyser.getByteTimeDomainData(dataArray);

      ctx.lineWidth = 2;
      ctx.strokeStyle = '#10b981';
      ctx.beginPath();

      const sliceW = (w * 1.0) / bufferLen;
      let x = 0;

      for (let i = 0; i < bufferLen; i++) {
        const v = dataArray[i] / 128.0;
        const y = v * (h / 2);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
        x += sliceW;
      }

      ctx.lineTo(w, h / 2);
      ctx.stroke();
    } else {
      // Idle Flat Line
      ctx.lineWidth = 1;
      ctx.strokeStyle = 'rgba(16, 185, 129, 0.3)';
      ctx.beginPath();
      ctx.moveTo(0, h / 2);
      ctx.lineTo(w, h / 2);
      ctx.stroke();
    }
  }

  draw();
}

/* -------------------------------------------------------------------
   3. INTERACTIVE OTO.INI WAVEFORM CALIBRATOR (CANVAS)
   ------------------------------------------------------------------- */
function initOtoCalibrator() {
  const canvas = document.getElementById('calibratorCanvas');
  if (!canvas) return;

  const ctx = canvas.getContext('2d');
  const pOffset = document.getElementById('paramOffset');
  const pConsonant = document.getElementById('paramConsonant');
  const pPreut = document.getElementById('paramPreutterance');
  const pOverlap = document.getElementById('paramOverlap');
  const pCutoff = document.getElementById('paramCutoff');

  const vOffset = document.getElementById('valOffset');
  const vConsonant = document.getElementById('valConsonant');
  const vPreut = document.getElementById('valPreutterance');
  const vOverlap = document.getElementById('valOverlap');
  const vCutoff = document.getElementById('valCutoff');

  function update() {
    const offset = parseInt(pOffset.value, 10);
    const consonant = parseInt(pConsonant.value, 10);
    const preut = parseInt(pPreut.value, 10);
    const overlap = parseInt(pOverlap.value, 10);
    const cutoff = parseInt(pCutoff.value, 10);

    if (vOffset) vOffset.textContent = `${offset} ms`;
    if (vConsonant) vConsonant.textContent = `${consonant} ms`;
    if (vPreut) vPreut.textContent = `${preut} ms`;
    if (vOverlap) vOverlap.textContent = `${overlap} ms`;
    if (vCutoff) vCutoff.textContent = `${cutoff} ms`;

    drawWaveform(offset, consonant, preut, overlap, cutoff);
  }

  function drawWaveform(offsetMs, consonantMs, preutMs, overlapMs, cutoffMs) {
    const w = canvas.width;
    const h = canvas.height;
    const totalMs = 600;

    ctx.fillStyle = '#06080c';
    ctx.fillRect(0, 0, w, h);

    const msToX = (ms) => (ms / totalMs) * w;

    const xOffset = msToX(offsetMs);
    const xConsonant = msToX(consonantMs);
    const xPreut = msToX(preutMs);
    const xOverlap = msToX(offsetMs + overlapMs);
    const xCutoff = msToX(totalMs + cutoffMs);

    // 1. Offset blanking
    ctx.fillStyle = 'rgba(239, 68, 68, 0.15)';
    ctx.fillRect(0, 0, xOffset, h);

    // 2. Consonant fixed
    ctx.fillStyle = 'rgba(245, 158, 11, 0.18)';
    ctx.fillRect(xOffset, 0, Math.max(0, xConsonant - xOffset), h);

    // 3. Overlap blend
    ctx.fillStyle = 'rgba(139, 92, 246, 0.18)';
    ctx.fillRect(xOffset, 0, Math.max(0, xOverlap - xOffset), h);

    // 4. Cutoff blanking
    ctx.fillStyle = 'rgba(107, 114, 128, 0.25)';
    ctx.fillRect(xCutoff, 0, Math.max(0, w - xCutoff), h);

    // Grid center line
    ctx.strokeStyle = '#1e2430';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();

    // Waveform
    ctx.strokeStyle = '#34d399';
    ctx.lineWidth = 1.5;
    ctx.beginPath();

    const midY = h / 2;
    for (let x = 0; x < w; x++) {
      const ms = (x / w) * totalMs;
      let amp = 0;

      if (ms < offsetMs || ms > (totalMs + cutoffMs)) {
        amp = (Math.random() - 0.5) * 4;
      } else if (ms < consonantMs) {
        const env = Math.sin(((ms - offsetMs) / (consonantMs - offsetMs)) * Math.PI);
        const noise = (Math.random() - 0.5) * 45;
        const transient = Math.sin(ms * 0.4) * 35;
        amp = (noise + transient) * env;
      } else {
        const t = (ms - consonantMs) * 0.15;
        const f0 = Math.sin(t * 1.5) * 30;
        const f1 = Math.sin(t * 4.5) * 20;
        const f2 = Math.sin(t * 9.0) * 10;
        amp = f0 + f1 + f2;
      }

      const y = midY + amp;
      if (x === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();

    // Markers
    function drawMarker(x, color, label) {
      ctx.strokeStyle = color;
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(x, 0);
      ctx.lineTo(x, h);
      ctx.stroke();

      ctx.fillStyle = color;
      ctx.font = '11px "JetBrains Mono", monospace';
      ctx.fillText(label, x + 4, 16);
    }

    drawMarker(xOffset, '#ef4444', 'Offset');
    drawMarker(xConsonant, '#f59e0b', 'Consonant');
    drawMarker(xPreut, '#3b82f6', 'Preutterance');
    drawMarker(xOverlap, '#8b5cf6', 'Overlap');
    drawMarker(xCutoff, '#94a3b8', 'Cutoff');
  }

  [pOffset, pConsonant, pPreut, pOverlap, pCutoff].forEach(input => {
    if (input) input.addEventListener('input', update);
  });

  update();
}

/* -------------------------------------------------------------------
   4. THEME GALLERY TABS
   ------------------------------------------------------------------- */
function initThemeGallery() {
  const tabs = document.querySelectorAll('.theme-tab-btn');
  const img = document.getElementById('themePreviewImg');
  const nameEl = document.getElementById('currentThemeName');
  const descEl = document.getElementById('currentThemeDesc');

  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      const src = tab.getAttribute('data-img');
      const name = tab.getAttribute('data-name');
      const desc = tab.getAttribute('data-desc');

      if (img) img.src = src;
      if (nameEl) nameEl.textContent = `Tema ${name}`;
      if (descEl) descEl.textContent = desc;
    });
  });
}

/* -------------------------------------------------------------------
   5. SOURCE COMPILATION TABS & COPY
   ------------------------------------------------------------------- */
const OS_BUILD_COMMANDS = {
  ubuntu: {
    title: 'Ubuntu / Debian / Linux Mint / Pop!_OS',
    code: `# Instalar dependências do ALSA e bibliotecas gráficas
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libasound2-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libfontconfig1-dev

# Clonar o repositório
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu

# Compilar em modo release com otimização total
cargo build --release

# Executar o binário
./target/release/kamafeu`
  },
  fedora: {
    title: 'Fedora / RHEL / CentOS Stream',
    code: `# Instalar dependências no Fedora
sudo dnf install -y gcc pkg-config alsa-lib-devel libxcb-devel libxkbcommon-devel fontconfig-devel

# Clonar e compilar
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu
cargo build --release

# Executar
./target/release/kamafeu`
  },
  arch: {
    title: 'Arch Linux / Manjaro',
    code: `# Instalar dependências no Arch Linux
sudo pacman -S --needed base-devel pkgconf alsa-lib libxcb libxkbcommon fontconfig

# Clonar e compilar
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu
cargo build --release

# Executar
./target/release/kamafeu`
  },
  macos: {
    title: 'macOS (Apple Silicon M1/M2/M3/M4 & Intel)',
    code: `# Instalar Xcode Command Line Tools (Metal e CoreAudio são nativos)
xcode-select --install

# Clonar o repositório
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu

# Compilar com aceleração nativa Metal/Cocoa
cargo build --release

# Executar o binário do Kamafeu
./target/release/kamafeu`
  },
  windows: {
    title: 'Windows 10/11 (MSVC C++ Toolchain)',
    code: `# 1. Instale o Visual Studio Build Tools com a opção "Desktop C++"
# 2. No terminal PowerShell:
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu

# Compilar em modo release
cargo build --release

# Executar
.\\target\\release\\kamafeu.exe`
  },
  android: {
    title: 'Android APK (Cross-compilation via cargo-apk)',
    code: `# Instalar cargo-apk e target ARM64
cargo install cargo-apk
rustup target add aarch64-linux-android

# Compilar o APK com motores nativos VENUS e Andromeda
cargo apk build --lib --release --target aarch64-linux-android

# O APK gerado estará em:
# target/release/apk/kamafeu.apk`
  }
};

function initCompilationTabs() {
  const tabs = document.querySelectorAll('.t-tab');
  const titleEl = document.getElementById('terminalOsTitle');
  const codeEl = document.querySelector('#terminalCode code');
  const copyBtn = document.getElementById('copyTerminalBtn');

  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      const osKey = tab.getAttribute('data-os');
      const data = OS_BUILD_COMMANDS[osKey];
      if (data) {
        if (titleEl) titleEl.textContent = data.title;
        if (codeEl) codeEl.textContent = data.code;
      }
    });
  });

  if (copyBtn && codeEl) {
    copyBtn.addEventListener('click', () => {
      navigator.clipboard.writeText(codeEl.textContent).then(() => {
        copyBtn.textContent = 'Copiado!';
        setTimeout(() => {
          copyBtn.textContent = 'Copiar';
        }, 2000);
      });
    });
  }
}

/* -------------------------------------------------------------------
   6. MOBILE NAV MENU
   ------------------------------------------------------------------- */
function initMobileNav() {
  const btn = document.getElementById('mobileMenuBtn');
  const nav = document.getElementById('navLinks');
  if (!btn || !nav) return;

  btn.addEventListener('click', () => {
    nav.classList.toggle('show');
  });

  nav.querySelectorAll('a').forEach(link => {
    link.addEventListener('click', () => {
      nav.classList.remove('show');
    });
  });
}
