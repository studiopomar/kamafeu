/* ===================================================================
   KAMAFEU STUDIO - WORKSTATION PORTAL JAVASCRIPT
   WebAudio Vocal Synthesizer with Audible Consonants & Formant Vowels
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

// Comprehensive Vowel Formant Bank (F1, F2, F3 + Nasal resonance)
const VOWEL_BANK = {
  // Oral Open & Closed Vowels
  a:        { f1: 820, f2: 1250, f3: 2550, q1: 6, q2: 6, q3: 8, isNasal: false },
  e_open:   { f1: 620, f2: 1850, f3: 2650, q1: 6, q2: 7, q3: 8, isNasal: false }, // [ ɛ ] (é)
  e_closed: { f1: 440, f2: 1950, f3: 2600, q1: 6, q2: 7, q3: 8, isNasal: false }, // [ e ] (ê)
  i:        { f1: 300, f2: 2350, f3: 3100, q1: 7, q2: 8, q3: 9, isNasal: false },
  o_open:   { f1: 600, f2: 1050, f3: 2450, q1: 6, q2: 6, q3: 7, isNasal: false }, // [ ɔ ] (ó)
  o_closed: { f1: 450, f2: 900,  f3: 2400, q1: 6, q2: 6, q3: 7, isNasal: false }, // [ o ] (ô)
  u:        { f1: 320, f2: 800,  f3: 2250, q1: 7, q2: 6, q3: 7, isNasal: false },
  schwa:    { f1: 500, f2: 1500, f3: 2500, q1: 5, q2: 5, q3: 6, isNasal: false }, // [ ə ]

  // Nasal Vowels (Português Brasileiro / BRAPA)
  an:       { f1: 650, f2: 1300, f3: 2400, q1: 4, q2: 5, q3: 6, isNasal: true, nasalFreq: 260 },
  en:       { f1: 450, f2: 1750, f3: 2500, q1: 4, q2: 5, q3: 6, isNasal: true, nasalFreq: 280 },
  in:       { f1: 320, f2: 2100, f3: 2800, q1: 5, q2: 6, q3: 7, isNasal: true, nasalFreq: 300 },
  on:       { f1: 480, f2: 950,  f3: 2300, q1: 4, q2: 5, q3: 6, isNasal: true, nasalFreq: 250 },
  un:       { f1: 340, f2: 850,  f3: 2150, q1: 5, q2: 5, q3: 6, isNasal: true, nasalFreq: 270 }
};

let audioCtx = null;
let activeVowelGain = null;
let activeOsc = null;
let activeSubOsc = null;
let activeLfo = null;
let activeConsonantNodes = [];
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
    synthAnalyser.connect(audioCtx.destination);
  }
}

// Generate White Noise Buffer
let whiteNoiseBuffer = null;
function getWhiteNoiseBuffer() {
  ensureAudioContext();
  if (!whiteNoiseBuffer) {
    const bufferSize = audioCtx.sampleRate * 2;
    whiteNoiseBuffer = audioCtx.createBuffer(1, bufferSize, audioCtx.sampleRate);
    const data = whiteNoiseBuffer.getChannelData(0);
    for (let i = 0; i < bufferSize; i++) {
      data[i] = Math.random() * 2 - 1;
    }
  }
  return whiteNoiseBuffer;
}

function initVocalSynthKeyboard() {
  const keys = document.querySelectorAll('.piano-keyboard .key');
  const consonantSelect = document.getElementById('synthConsonant');
  const vowelSelect = document.getElementById('synthVowel');
  const pitchSlider = document.getElementById('synthPitchBend');
  const pitchVal = document.getElementById('synthPitchVal');
  const vibratoBtn = document.getElementById('synthVibratoBtn');
  const playTwinkleBtn = document.getElementById('btnPlayTwinkle');
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
      const c = consonantSelect ? consonantSelect.value : 'none';
      const v = vowelSelect ? vowelSelect.value : 'e_closed';
      playSynthVoice(freq, note, c, v);
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

  // Melodies & Preset Songs
  if (playTwinkleBtn) {
    playTwinkleBtn.addEventListener('click', () => {
      // "Brilha, brilha, estrelinha / Quero ver você brilhar" (Partitura completa com consoantes e vogais)
      playVocalSequence([
        { note: 'C4', freq: 261.63, cons: 'b',  vowel: 'i',        lyric: 'Bri-',  dur: 360 },
        { note: 'C4', freq: 261.63, cons: 'lh', vowel: 'a',        lyric: 'lha,',  dur: 360 },
        { note: 'G4', freq: 392.00, cons: 'b',  vowel: 'i',        lyric: 'bri-',  dur: 360 },
        { note: 'G4', freq: 392.00, cons: 'lh', vowel: 'a',        lyric: 'lha,',  dur: 360 },
        { note: 'A4', freq: 440.00, cons: 's',  vowel: 'e_closed', lyric: 'es-',   dur: 360 },
        { note: 'A4', freq: 440.00, cons: 't',  vowel: 'e_open',   lyric: 'tre-',  dur: 360 },
        { note: 'G4', freq: 392.00, cons: 'l',  vowel: 'i',        lyric: 'li-',   dur: 640 },
        { note: 'F4', freq: 349.23, cons: 'nh', vowel: 'a',        lyric: 'nha,',  dur: 360 },
        { note: 'F4', freq: 349.23, cons: 'k',  vowel: 'e_open',   lyric: 'que-',  dur: 360 },
        { note: 'E4', freq: 329.63, cons: 'r',  vowel: 'o_closed', lyric: 'ro',    dur: 360 },
        { note: 'E4', freq: 329.63, cons: 'v',  vowel: 'e_open',   lyric: 'ver',   dur: 360 },
        { note: 'D4', freq: 293.66, cons: 'v',  vowel: 'o_closed', lyric: 'vo-',   dur: 360 },
        { note: 'D4', freq: 293.66, cons: 's',  vowel: 'e_closed', lyric: 'cê',    dur: 360 },
        { note: 'C4', freq: 261.63, cons: 'b',  vowel: 'i',        lyric: 'bri-',  dur: 420 },
        { note: 'C4', freq: 261.63, cons: 'lh', vowel: 'a',        lyric: 'lhar! ✨', dur: 850 }
      ]);
    });
  }

  if (playKamafeuBtn) {
    playKamafeuBtn.addEventListener('click', () => {
      playVocalSequence([
        { note: 'C4', freq: 261.63, cons: 'k',  vowel: 'a',        lyric: 'Ka',   dur: 340 },
        { note: 'D4', freq: 293.66, cons: 'm',  vowel: 'a',        lyric: 'ma',   dur: 340 },
        { note: 'E4', freq: 329.63, cons: 'f',  vowel: 'e_closed', lyric: 'feu',  dur: 480 },
        { note: 'G4', freq: 392.00, cons: 's',  vowel: 'u',        lyric: 'Stu-', dur: 320 },
        { note: 'A4', freq: 440.00, cons: 'd',  vowel: 'i',        lyric: 'dio',  dur: 750 }
      ]);
    });
  }

  if (playPomarBtn) {
    playPomarBtn.addEventListener('click', () => {
      playVocalSequence([
        { note: 'E4', freq: 329.63, cons: 's',  vowel: 'u',        lyric: 'Stu-', dur: 300 },
        { note: 'G4', freq: 392.00, cons: 'd',  vowel: 'i',        lyric: 'dio',  dur: 300 },
        { note: 'C5', freq: 523.25, cons: 'p',  vowel: 'o_closed', lyric: 'Po-',  dur: 400 },
        { note: 'G4', freq: 392.00, cons: 'm',  vowel: 'a',        lyric: 'mar 🍊', dur: 750 }
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

// Synthesize Note with Explicit Consonant Acoustics + Formant Vowel
function playSynthVoice(baseFreq, noteName, cons = 'none', vowelKey = 'e_closed') {
  ensureAudioContext();
  stopSynthVoice();

  const formants = VOWEL_BANK[vowelKey] || VOWEL_BANK.a;

  const pitchSlider = document.getElementById('synthPitchBend');
  const cents = pitchSlider ? parseInt(pitchSlider.value, 10) : 0;
  const targetFreq = baseFreq * Math.pow(2, cents / 1200);

  const now = audioCtx.currentTime;

  // Determine consonant timing (preutterance duration)
  const hasConsonant = cons && cons !== 'none';
  const consonantDuration = hasConsonant ? getConsonantDuration(cons) : 0;
  const vowelStartTime = now + (hasConsonant ? consonantDuration * 0.4 : 0);

  // 1. Play Explicit Acoustic Consonant
  if (hasConsonant) {
    playAcousticConsonant(cons, now, targetFreq);
  }

  // 2. Play Formant Vowel
  activeVowelGain = audioCtx.createGain();
  activeVowelGain.gain.setValueAtTime(0.0001, now);

  if (hasConsonant) {
    // Fade vowel in right at the consonant release
    activeVowelGain.gain.setValueAtTime(0.0001, vowelStartTime);
    activeVowelGain.gain.exponentialRampToValueAtTime(0.48, vowelStartTime + 0.04);
  } else {
    // Immediate smooth attack for pure vowel
    activeVowelGain.gain.exponentialRampToValueAtTime(0.48, now + 0.03);
  }

  // Glottal Sawtooth Waveform (Vocal cord excitation)
  activeOsc = audioCtx.createOscillator();
  activeOsc.type = 'sawtooth';
  activeOsc.frequency.setValueAtTime(targetFreq, now);
  activeOsc.datasetBaseFreq = baseFreq;

  // Sub oscillator for chest resonance
  activeSubOsc = audioCtx.createOscillator();
  activeSubOsc.type = 'sine';
  activeSubOsc.frequency.setValueAtTime(targetFreq * 0.5, now);
  const subGain = audioCtx.createGain();
  subGain.gain.setValueAtTime(0.12, now);
  activeSubOsc.connect(subGain);

  // Vibrato LFO (6.0 Hz)
  if (isVibratoOn) {
    activeLfo = audioCtx.createOscillator();
    activeLfo.frequency.setValueAtTime(6.0, now);
    const lfoGain = audioCtx.createGain();
    lfoGain.gain.setValueAtTime(5.5, now);
    activeLfo.connect(lfoGain);
    lfoGain.connect(activeOsc.frequency);
    activeLfo.start(vowelStartTime);
  }

  // Formant Filter Bank (F1, F2, F3 parallel bandpasses)
  const fMix = audioCtx.createGain();
  fMix.gain.setValueAtTime(0.9, now);

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

  // Nasal Resonator for Nasal Vowels (ã, ẽ, ĩ, õ, ũ)
  if (formants.isNasal && formants.nasalFreq) {
    const nasalFilter = audioCtx.createBiquadFilter();
    nasalFilter.type = 'peaking';
    nasalFilter.frequency.setValueAtTime(formants.nasalFreq, now);
    nasalFilter.Q.setValueAtTime(3.5, now);
    nasalFilter.gain.setValueAtTime(8, now);
    fMix.connect(nasalFilter);
    nasalFilter.connect(activeVowelGain);
  } else {
    fMix.connect(activeVowelGain);
  }

  activeVowelGain.connect(synthAnalyser);

  activeOsc.start(vowelStartTime);
  activeSubOsc.start(vowelStartTime);

  const statusInfo = document.getElementById('synthStatusInfo');
  if (statusInfo) {
    const consLabel = hasConsonant ? `[ ${cons.toUpperCase()} ] + ` : '';
    statusInfo.textContent = `Sintetizando: ${noteName} (${targetFreq.toFixed(1)} Hz) — ${consLabel}Vogal [ ${vowelKey.toUpperCase()} ]`;
  }
}

function getConsonantDuration(cons) {
  switch (cons) {
    case 's': case 'z': case 'sh': case 'ch': return 0.11; // Fricatives 110ms
    case 'f': case 'v': return 0.09;
    case 'm': case 'n': case 'nh': return 0.08; // Nasals 80ms
    case 'l': case 'lh': case 'r': return 0.06; // Liquids 60ms
    default: return 0.045; // Plosives (p, t, k, b, d, g) 45ms
  }
}

// Generate Realistic Audible Acoustic Consonant
function playAcousticConsonant(cons, t0, pitchFreq) {
  const noiseBuf = getWhiteNoiseBuffer();
  const consMasterGain = audioCtx.createGain();
  consMasterGain.connect(synthAnalyser);

  activeConsonantNodes.push(consMasterGain);

  switch (cons) {
    // -------------------------------------------------------------
    // PLOSIVES (p, t, k, b, d, g): Sharp pop transient + burst noise
    // -------------------------------------------------------------
    case 'k':
    case 'g': {
      // Velar burst noise centered at 2.2 kHz
      const noiseSource = audioCtx.createBufferSource();
      noiseSource.buffer = noiseBuf;
      const bp = audioCtx.createBiquadFilter();
      bp.type = 'bandpass';
      bp.frequency.setValueAtTime(2200, t0);
      bp.Q.setValueAtTime(4.0, t0);

      const gain = audioCtx.createGain();
      gain.gain.setValueAtTime(0.85, t0);
      gain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.045);

      noiseSource.connect(bp);
      bp.connect(gain);
      gain.connect(consMasterGain);
      noiseSource.start(t0);
      noiseSource.stop(t0 + 0.05);

      // Transient Pop Pitch Drop
      const popOsc = audioCtx.createOscillator();
      popOsc.type = 'triangle';
      popOsc.frequency.setValueAtTime(cons === 'g' ? 350 : 600, t0);
      popOsc.frequency.exponentialRampToValueAtTime(100, t0 + 0.03);
      const popGain = audioCtx.createGain();
      popGain.gain.setValueAtTime(0.6, t0);
      popGain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.03);
      popOsc.connect(popGain);
      popGain.connect(consMasterGain);
      popOsc.start(t0);
      popOsc.stop(t0 + 0.035);
      break;
    }

    case 't':
    case 'd': {
      // Alveolar high frequency click (4.2 kHz)
      const noiseSource = audioCtx.createBufferSource();
      noiseSource.buffer = noiseBuf;
      const bp = audioCtx.createBiquadFilter();
      bp.type = 'bandpass';
      bp.frequency.setValueAtTime(4500, t0);
      bp.Q.setValueAtTime(5.0, t0);

      const gain = audioCtx.createGain();
      gain.gain.setValueAtTime(0.9, t0);
      gain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.04);

      noiseSource.connect(bp);
      bp.connect(gain);
      gain.connect(consMasterGain);
      noiseSource.start(t0);
      noiseSource.stop(t0 + 0.045);

      const popOsc = audioCtx.createOscillator();
      popOsc.type = 'sine';
      popOsc.frequency.setValueAtTime(800, t0);
      popOsc.frequency.exponentialRampToValueAtTime(150, t0 + 0.025);
      const popGain = audioCtx.createGain();
      popGain.gain.setValueAtTime(0.55, t0);
      popGain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.025);
      popOsc.connect(popGain);
      popGain.connect(consMasterGain);
      popOsc.start(t0);
      popOsc.stop(t0 + 0.03);
      break;
    }

    case 'p':
    case 'b': {
      // Bilabial low pop + soft burst
      const popOsc = audioCtx.createOscillator();
      popOsc.type = 'sine';
      popOsc.frequency.setValueAtTime(320, t0);
      popOsc.frequency.exponentialRampToValueAtTime(60, t0 + 0.04);
      const popGain = audioCtx.createGain();
      popGain.gain.setValueAtTime(0.95, t0);
      popGain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.04);
      popOsc.connect(popGain);
      popGain.connect(consMasterGain);
      popOsc.start(t0);
      popOsc.stop(t0 + 0.045);

      const noiseSource = audioCtx.createBufferSource();
      noiseSource.buffer = noiseBuf;
      const lp = audioCtx.createBiquadFilter();
      lp.type = 'lowpass';
      lp.frequency.setValueAtTime(1200, t0);
      const noiseGain = audioCtx.createGain();
      noiseGain.gain.setValueAtTime(0.5, t0);
      noiseGain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.035);
      noiseSource.connect(lp);
      lp.connect(noiseGain);
      noiseGain.connect(consMasterGain);
      noiseSource.start(t0);
      noiseSource.stop(t0 + 0.04);
      break;
    }

    // -------------------------------------------------------------
    // FRICATIVES (s, z, sh, ch, f, v): Textured audible noise band
    // -------------------------------------------------------------
    case 's':
    case 'z': {
      // High-frequency sibilant hiss (5.5 kHz - 9 kHz)
      const noiseSource = audioCtx.createBufferSource();
      noiseSource.buffer = noiseBuf;
      const hp = audioCtx.createBiquadFilter();
      hp.type = 'highpass';
      hp.frequency.setValueAtTime(4800, t0);

      const bp = audioCtx.createBiquadFilter();
      bp.type = 'bandpass';
      bp.frequency.setValueAtTime(6500, t0);
      bp.Q.setValueAtTime(2.0, t0);

      const gain = audioCtx.createGain();
      gain.gain.setValueAtTime(0.85, t0);
      gain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.11);

      noiseSource.connect(hp);
      hp.connect(bp);
      bp.connect(gain);
      gain.connect(consMasterGain);
      noiseSource.start(t0);
      noiseSource.stop(t0 + 0.12);

      // Voiced 'z' hum
      if (cons === 'z') {
        const hum = audioCtx.createOscillator();
        hum.type = 'sawtooth';
        hum.frequency.setValueAtTime(pitchFreq, t0);
        const humGain = audioCtx.createGain();
        humGain.gain.setValueAtTime(0.3, t0);
        humGain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.1);
        hum.connect(humGain);
        humGain.connect(consMasterGain);
        hum.start(t0);
        hum.stop(t0 + 0.11);
      }
      break;
    }

    case 'sh':
    case 'ch': {
      // Post-alveolar fricative [ʃ] (2.8 kHz - 4.5 kHz)
      const noiseSource = audioCtx.createBufferSource();
      noiseSource.buffer = noiseBuf;
      const bp = audioCtx.createBiquadFilter();
      bp.type = 'bandpass';
      bp.frequency.setValueAtTime(3200, t0);
      bp.Q.setValueAtTime(1.8, t0);

      const gain = audioCtx.createGain();
      gain.gain.setValueAtTime(0.85, t0);
      gain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.10);

      noiseSource.connect(bp);
      bp.connect(gain);
      gain.connect(consMasterGain);
      noiseSource.start(t0);
      noiseSource.stop(t0 + 0.11);
      break;
    }

    case 'f':
    case 'v': {
      // Labiodental soft turbulent air
      const noiseSource = audioCtx.createBufferSource();
      noiseSource.buffer = noiseBuf;
      const bp = audioCtx.createBiquadFilter();
      bp.type = 'bandpass';
      bp.frequency.setValueAtTime(2400, t0);
      bp.Q.setValueAtTime(1.2, t0);

      const gain = audioCtx.createGain();
      gain.gain.setValueAtTime(0.65, t0);
      gain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.08);

      noiseSource.connect(bp);
      bp.connect(gain);
      gain.connect(consMasterGain);
      noiseSource.start(t0);
      noiseSource.stop(t0 + 0.09);

      if (cons === 'v') {
        const hum = audioCtx.createOscillator();
        hum.type = 'sine';
        hum.frequency.setValueAtTime(pitchFreq, t0);
        const humGain = audioCtx.createGain();
        humGain.gain.setValueAtTime(0.35, t0);
        humGain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.08);
        hum.connect(humGain);
        humGain.connect(consMasterGain);
        hum.start(t0);
        hum.stop(t0 + 0.09);
      }
      break;
    }

    // -------------------------------------------------------------
    // NASALS (m, n, nh): Low nasal murmur hum
    // -------------------------------------------------------------
    case 'm':
    case 'n':
    case 'nh': {
      const murmur = audioCtx.createOscillator();
      murmur.type = 'triangle';
      murmur.frequency.setValueAtTime(pitchFreq, t0);

      const lp = audioCtx.createBiquadFilter();
      lp.type = 'lowpass';
      lp.frequency.setValueAtTime(cons === 'nh' ? 450 : 280, t0);
      lp.Q.setValueAtTime(3.0, t0);

      const gain = audioCtx.createGain();
      gain.gain.setValueAtTime(0.7, t0);
      gain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.08);

      murmur.connect(lp);
      lp.connect(gain);
      gain.connect(consMasterGain);
      murmur.start(t0);
      murmur.stop(t0 + 0.09);
      break;
    }

    // -------------------------------------------------------------
    // LIQUIDS (l, lh, r): Formant transition glide & alveolar tap
    // -------------------------------------------------------------
    case 'l':
    case 'lh': {
      const glideOsc = audioCtx.createOscillator();
      glideOsc.type = 'sawtooth';
      glideOsc.frequency.setValueAtTime(pitchFreq, t0);

      const bp = audioCtx.createBiquadFilter();
      bp.type = 'bandpass';
      bp.frequency.setValueAtTime(cons === 'lh' ? 2200 : 1300, t0);
      bp.Q.setValueAtTime(4.0, t0);

      const gain = audioCtx.createGain();
      gain.gain.setValueAtTime(0.65, t0);
      gain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.06);

      glideOsc.connect(bp);
      bp.connect(gain);
      gain.connect(consMasterGain);
      glideOsc.start(t0);
      glideOsc.stop(t0 + 0.07);
      break;
    }

    case 'r': {
      // Alveolar tap (rapid modulation pulse)
      const tapOsc = audioCtx.createOscillator();
      tapOsc.type = 'sawtooth';
      tapOsc.frequency.setValueAtTime(pitchFreq, t0);

      const tremolo = audioCtx.createOscillator();
      tremolo.frequency.setValueAtTime(30, t0); // 30 Hz tongue tap
      const tremoloGain = audioCtx.createGain();
      tremoloGain.gain.setValueAtTime(0.5, t0);
      tremolo.connect(tremoloGain.gain);

      const bp = audioCtx.createBiquadFilter();
      bp.type = 'bandpass';
      bp.frequency.setValueAtTime(1600, t0);
      bp.Q.setValueAtTime(3.0, t0);

      const gain = audioCtx.createGain();
      gain.gain.setValueAtTime(0.6, t0);
      gain.gain.exponentialRampToValueAtTime(0.001, t0 + 0.05);

      tapOsc.connect(bp);
      bp.connect(tremoloGain);
      tremoloGain.connect(gain);
      gain.connect(consMasterGain);

      tapOsc.start(t0);
      tremolo.start(t0);
      tapOsc.stop(t0 + 0.06);
      tremolo.stop(t0 + 0.06);
      break;
    }
  }
}

function stopSynthVoice() {
  if (activeVowelGain && audioCtx) {
    const now = audioCtx.currentTime;
    activeVowelGain.gain.cancelScheduledValues(now);
    activeVowelGain.gain.setValueAtTime(activeVowelGain.gain.value, now);
    activeVowelGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.05);

    const oldOsc = activeOsc;
    const oldSub = activeSubOsc;
    const oldLfo = activeLfo;
    setTimeout(() => {
      try {
        if (oldOsc) oldOsc.stop();
        if (oldSub) oldSub.stop();
        if (oldLfo) oldLfo.stop();
      } catch (e) {}
    }, 60);

    activeOsc = null;
    activeSubOsc = null;
    activeLfo = null;
    activeVowelGain = null;
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

      const consonantSelect = document.getElementById('synthConsonant');
      if (consonantSelect && item.cons) consonantSelect.value = item.cons;

      const vowelSelect = document.getElementById('synthVowel');
      if (vowelSelect && item.vowel) vowelSelect.value = item.vowel;

      playSynthVoice(item.freq, `${item.note} ("${item.lyric}")`, item.cons || 'none', item.vowel || 'a');

      const stopT = setTimeout(() => {
        stopSynthVoice();
        if (keyEl) keyEl.classList.remove('playing');
        if (idx === notes.length - 1) {
          const statusInfo = document.getElementById('synthStatusInfo');
          if (statusInfo) statusInfo.textContent = 'Música finalizada ✨';
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

    if (synthAnalyser && (activeVowelGain || activeConsonantNodes.length > 0)) {
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
