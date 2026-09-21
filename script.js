/* ===================================================================
   KAMAFEU STUDIO - INTERACTIVE JAVASCRIPT ENGINE
   WebAudio Formant Synthesis, Piano Roll, Oscilloscope & Dynamic UI
   =================================================================== */

document.addEventListener('DOMContentLoaded', () => {
  initThemeEngine();
  initWebAudioPlayground();
  initPipelineExplorer();
  initThemeGallery();
  initShortcutsSearch();
  initCompilationTabs();
  initCopyButtons();
  initMobileMenu();
});

/* -------------------------------------------------------------------
   1. THEME ENGINE & LIVE SWITCHER
   ------------------------------------------------------------------- */
const THEME_DATA = {
  'pomar-neon': { name: 'Pomar Neon (Esmeralda)', dotClass: 'dot-pomar', img: 'assets/themes/theme_pomar_neon.png', desc: 'Paleta oficial do Studio Pomar com tons esmeralda de alto contraste, linhas de grade neon e piano roll legível para longas sessões de composição.' },
  'melodyne-gold': { name: 'Melodyne Gold (Âmbar)', dotClass: 'dot-melodyne', img: 'assets/themes/theme_melodyne_gold.png', desc: 'Inspirado nos editores analógicos e clássicos de afinação manual, com tons de âmbar quente, caramelo e alto conforto visual.' },
  'cyberpunk': { name: 'Cyberpunk (Synthwave)', dotClass: 'dot-cyberpunk', img: 'assets/themes/theme_cyberpunk.png', desc: 'Estética neon vibrante com notas magenta brilhantes, grade ciano profunda e forte atmosfera synthwave/retro-futurista.' },
  'nordic-slate': { name: 'Nordic Slate (Clean Dark)', dotClass: 'dot-nordic', img: 'assets/themes/theme_nordic_slate.png', desc: 'Tema escuro neutro e minimalista construído em tons de ardósia e titânio para máxima concentração e precisão técnica.' },
  'mikan-teal': { name: 'Voz-a-loide Teal (Mikan)', dotClass: 'dot-mikan', img: 'assets/themes/theme_mikan_teal.png', desc: 'Cores inspiradas na cultura clássica de cantores virtuais, combinando ciano elétrico e detalhes alaranjados vibrantes.' }
};

function initThemeEngine() {
  const root = document.documentElement;
  const themeBtn = document.getElementById('themeBtn');
  const themeDropdown = document.getElementById('themeDropdown');
  const activeDot = document.getElementById('activeThemeDot');
  const activeName = document.getElementById('activeThemeName');
  const themeOpts = document.querySelectorAll('.theme-opt');

  // Load saved theme or default
  const savedTheme = localStorage.getItem('kamafeu-theme') || 'pomar-neon';
  applyTheme(savedTheme);

  // Toggle Dropdown
  if (themeBtn && themeDropdown) {
    themeBtn.addEventListener('click', (e) => {
      e.stopPropagation();
      const isOpen = themeDropdown.classList.contains('show');
      themeDropdown.classList.toggle('show', !isOpen);
      themeBtn.setAttribute('aria-expanded', !isOpen);
    });

    document.addEventListener('click', () => {
      themeDropdown.classList.remove('show');
      themeBtn.setAttribute('aria-expanded', 'false');
    });
  }

  // Option Click
  themeOpts.forEach(opt => {
    opt.addEventListener('click', () => {
      const themeKey = opt.getAttribute('data-theme');
      applyTheme(themeKey);
      if (themeDropdown) themeDropdown.classList.remove('show');
    });
  });

  function applyTheme(themeKey) {
    if (!THEME_DATA[themeKey]) themeKey = 'pomar-neon';
    root.setAttribute('data-theme', themeKey);
    localStorage.setItem('kamafeu-theme', themeKey);

    const info = THEME_DATA[themeKey];
    if (activeName) activeName.textContent = info.name.split(' ')[0] + ' ' + info.name.split(' ')[1];
    if (activeDot) {
      activeDot.className = 'theme-dot ' + info.dotClass;
    }

    themeOpts.forEach(opt => {
      opt.classList.toggle('active', opt.getAttribute('data-theme') === themeKey);
    });

    // Sync with gallery tabs if present
    const galleryTab = document.querySelector(`.gallery-tab[data-theme-id="${themeKey.replace('-', '_')}"]`);
    if (galleryTab) {
      document.querySelectorAll('.gallery-tab').forEach(t => t.classList.remove('active'));
      galleryTab.classList.add('active');
      updateGalleryDisplay(themeKey.replace('-', '_'));
    }
  }
}

/* -------------------------------------------------------------------
   2. WEBAUDIO FORMANT SYNTHESIS & PIANO ROLL PLAYGROUND
   ------------------------------------------------------------------- */
// Vowel formant frequencies [F1, F2, F3]
const VOWEL_FORMANTS = {
  a: { f1: 800, f2: 1200, f3: 2500, q1: 6, q2: 6, q3: 8 },
  e: { f1: 500, f2: 1800, f3: 2600, q1: 6, q2: 7, q3: 8 },
  i: { f1: 300, f2: 2200, f3: 3000, q1: 7, q2: 8, q3: 9 },
  o: { f1: 500, f2: 850,  f3: 2400, q1: 6, q2: 6, q3: 7 },
  u: { f1: 320, f2: 800,  f3: 2200, q1: 7, q2: 6, q3: 7 }
};

let audioCtx = null;
let activeOscillator = null;
let activeGain = null;
let formantFilters = [];
let vibratoLfo = null;
let vibratoGain = null;
let isVibratoEnabled = true;
let analyserNode = null;
let animFrameId = null;
let phraseTimeouts = [];

function initWebAudioPlayground() {
  const pianoKeys = document.querySelectorAll('.piano-key');
  const vowelSelect = document.getElementById('vowelSelect');
  const pitchBendSlider = document.getElementById('pitchBendSlider');
  const pitchBendVal = document.getElementById('pitchBendVal');
  const vibratoToggle = document.getElementById('vibratoToggle');
  const canvas = document.getElementById('oscilloscopeCanvas');
  const oscNoteDisplay = document.getElementById('oscNoteDisplay');
  const playKamafeuBtn = document.getElementById('playKamafeuPhrase');
  const playPomarBtn = document.getElementById('playPomarPhrase');
  const stopPlaybackBtn = document.getElementById('stopPlaybackBtn');

  // Pitch bend slider
  if (pitchBendSlider && pitchBendVal) {
    pitchBendSlider.addEventListener('input', (e) => {
      const val = parseInt(e.target.value, 10);
      pitchBendVal.textContent = `${val > 0 ? '+' : ''}${val} cents`;
      if (activeOscillator && audioCtx) {
        const baseFreq = parseFloat(activeOscillator.datasetFreq || 261.63);
        const bentFreq = baseFreq * Math.pow(2, val / 1200);
        activeOscillator.frequency.setTargetAtTime(bentFreq, audioCtx.currentTime, 0.02);
      }
    });
  }

  // Vibrato Toggle
  if (vibratoToggle) {
    vibratoToggle.addEventListener('click', () => {
      isVibratoEnabled = !isVibratoEnabled;
      vibratoToggle.classList.toggle('active', isVibratoEnabled);
      vibratoToggle.textContent = isVibratoEnabled ? '6.0 Hz / 45c' : 'Desativado';
      if (vibratoGain && audioCtx) {
        vibratoGain.gain.setValueAtTime(isVibratoEnabled ? 4.5 : 0, audioCtx.currentTime);
      }
    });
  }

  // Piano Keys Listeners
  pianoKeys.forEach(key => {
    const note = key.getAttribute('data-note');
    const freq = parseFloat(key.getAttribute('data-freq'));

    const startNote = (e) => {
      e.preventDefault();
      stopAllPhrases();
      playTone(freq, note);
      key.classList.add('playing');
    };

    const stopNote = (e) => {
      e.preventDefault();
      stopTone();
      key.classList.remove('playing');
    };

    key.addEventListener('mousedown', startNote);
    key.addEventListener('mouseup', stopNote);
    key.addEventListener('mouseleave', stopNote);

    // Touch support
    key.addEventListener('touchstart', startNote, { passive: false });
    key.addEventListener('touchend', stopNote, { passive: false });
  });

  // Melody Presets
  if (playKamafeuBtn) {
    playKamafeuBtn.addEventListener('click', () => {
      playPhraseSequence([
        { note: 'C4', freq: 261.63, vowel: 'a', lyric: 'Ka', dur: 320 },
        { note: 'D4', freq: 293.66, vowel: 'a', lyric: 'ma', dur: 320 },
        { note: 'E4', freq: 329.63, vowel: 'e', lyric: 'feu', dur: 450 },
        { note: 'G4', freq: 392.00, vowel: 'u', lyric: 'Stu-', dur: 300 },
        { note: 'A4', freq: 440.00, vowel: 'o', lyric: 'dio!', dur: 700 }
      ]);
    });
  }

  if (playPomarBtn) {
    playPomarBtn.addEventListener('click', () => {
      playPhraseSequence([
        { note: 'E4', freq: 329.63, vowel: 'u', lyric: 'Stu-', dur: 280 },
        { note: 'G4', freq: 392.00, vowel: 'i', lyric: 'dio', dur: 280 },
        { note: 'C5', freq: 523.25, vowel: 'o', lyric: 'Po-', dur: 380 },
        { note: 'G4', freq: 392.00, vowel: 'a', lyric: 'mar', dur: 700 }
      ]);
    });
  }

  if (stopPlaybackBtn) {
    stopPlaybackBtn.addEventListener('click', () => {
      stopAllPhrases();
      stopTone();
      if (oscNoteDisplay) oscNoteDisplay.textContent = 'Parado';
    });
  }

  // Setup Canvas Oscilloscope
  if (canvas) {
    setupOscilloscope(canvas);
  }
}

function ensureAudioContext() {
  if (!audioCtx) {
    const AudioContextClass = window.AudioContext || window.webkitAudioContext;
    audioCtx = new AudioContextClass();
  }
  if (audioCtx.state === 'suspended') {
    audioCtx.resume();
  }
  if (!analyserNode) {
    analyserNode = audioCtx.createAnalyser();
    analyserNode.fftSize = 1024;
  }
}

function playTone(baseFreq, noteName = '') {
  ensureAudioContext();
  stopTone();

  const vowelSelect = document.getElementById('vowelSelect');
  const vowel = vowelSelect ? vowelSelect.value : 'a';
  const formants = VOWEL_FORMANTS[vowel] || VOWEL_FORMANTS.a;

  const pitchBendSlider = document.getElementById('pitchBendSlider');
  const bendCents = pitchBendSlider ? parseInt(pitchBendSlider.value, 10) : 0;
  const targetFreq = baseFreq * Math.pow(2, bendCents / 1200);

  // Master Gain & Envelope
  activeGain = audioCtx.createGain();
  const now = audioCtx.currentTime;
  activeGain.gain.setValueAtTime(0.0001, now);
  // Attack (35ms)
  activeGain.gain.exponentialRampToValueAtTime(0.4, now + 0.035);

  // Primary Glottal Pulse Generator (Sawtooth waveform enriched with rich harmonics)
  activeOscillator = audioCtx.createOscillator();
  activeOscillator.type = 'sawtooth';
  activeOscillator.frequency.setValueAtTime(targetFreq, now);
  activeOscillator.datasetFreq = baseFreq;

  // Secondary Warmth Oscillator (Square 1 octave below mixed softly)
  const subOsc = audioCtx.createOscillator();
  subOsc.type = 'square';
  subOsc.frequency.setValueAtTime(targetFreq * 0.5, now);
  const subGain = audioCtx.createGain();
  subGain.gain.setValueAtTime(0.08, now);
  subOsc.connect(subGain);

  // Vibrato LFO
  if (isVibratoEnabled) {
    vibratoLfo = audioCtx.createOscillator();
    vibratoLfo.frequency.setValueAtTime(6.0, now); // 6 Hz vibrato
    vibratoGain = audioCtx.createGain();
    vibratoGain.gain.setValueAtTime(5.5, now);
    vibratoLfo.connect(vibratoGain);
    vibratoGain.connect(activeOscillator.frequency);
    vibratoLfo.start(now);
  }

  // Vocal Tract Formant Filter Bank (F1, F2, F3 parallel bandpasses)
  const formantMixGain = audioCtx.createGain();
  formantMixGain.gain.setValueAtTime(0.85, now);

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

  activeOscillator.connect(f1);
  activeOscillator.connect(f2);
  activeOscillator.connect(f3);
  subGain.connect(f1);

  f1.connect(formantMixGain);
  f2.connect(formantMixGain);
  f3.connect(formantMixGain);

  formantMixGain.connect(activeGain);
  activeGain.connect(analyserNode);
  analyserNode.connect(audioCtx.destination);

  activeOscillator.start(now);
  subOsc.start(now);

  activeOscillator.subOsc = subOsc;

  const oscNoteDisplay = document.getElementById('oscNoteDisplay');
  if (oscNoteDisplay) {
    oscNoteDisplay.textContent = `Sintetizando: ${noteName} (${targetFreq.toFixed(1)} Hz) — Vogal [ ${vowel.toUpperCase()} ] — VENUS Formant Bank`;
  }
}

function stopTone() {
  if (activeGain && audioCtx) {
    const now = audioCtx.currentTime;
    // Release (60ms smooth fade)
    activeGain.gain.cancelScheduledValues(now);
    activeGain.gain.setValueAtTime(activeGain.gain.value, now);
    activeGain.gain.exponentialRampToValueAtTime(0.0001, now + 0.06);

    const oldOsc = activeOscillator;
    const oldLfo = vibratoLfo;
    setTimeout(() => {
      try {
        if (oldOsc) {
          oldOsc.stop();
          if (oldOsc.subOsc) oldOsc.subOsc.stop();
        }
        if (oldLfo) oldLfo.stop();
      } catch (e) {}
    }, 70);

    activeOscillator = null;
    activeGain = null;
    vibratoLfo = null;
  }
}

function stopAllPhrases() {
  phraseTimeouts.forEach(t => clearTimeout(t));
  phraseTimeouts = [];
  document.querySelectorAll('.piano-key').forEach(k => k.classList.remove('playing'));
}

function playPhraseSequence(notes) {
  ensureAudioContext();
  stopAllPhrases();
  stopTone();

  let delay = 0;
  notes.forEach((item, index) => {
    const t = setTimeout(() => {
      // Highlight key
      document.querySelectorAll('.piano-key').forEach(k => k.classList.remove('playing'));
      const keyEl = document.querySelector(`.piano-key[data-note="${item.note}"]`);
      if (keyEl) keyEl.classList.add('playing');

      // Change vowel
      const vowelSelect = document.getElementById('vowelSelect');
      if (vowelSelect) vowelSelect.value = item.vowel;

      playTone(item.freq, `${item.note} ("${item.lyric}")`);

      const stopT = setTimeout(() => {
        stopTone();
        if (keyEl) keyEl.classList.remove('playing');
        if (index === notes.length - 1) {
          const oscNoteDisplay = document.getElementById('oscNoteDisplay');
          if (oscNoteDisplay) oscNoteDisplay.textContent = 'Sequência concluída — Pronto';
        }
      }, item.dur - 40);
      phraseTimeouts.push(stopT);
    }, delay);

    phraseTimeouts.push(t);
    delay += item.dur;
  });
}

function setupOscilloscope(canvas) {
  const ctx = canvas.getContext('2d');
  const bufferLength = 512;
  const dataArray = new Uint8Array(bufferLength);

  function draw() {
    animFrameId = requestAnimationFrame(draw);

    ctx.fillStyle = '#030508';
    ctx.fillRect(0, 0, canvas.width, canvas.height);

    // Draw Radar Grid lines
    ctx.lineWidth = 1;
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.05)';
    ctx.beginPath();
    for (let x = 0; x < canvas.width; x += 40) {
      ctx.moveTo(x, 0); ctx.lineTo(x, canvas.height);
    }
    for (let y = 0; y < canvas.height; y += 28) {
      ctx.moveTo(0, y); ctx.lineTo(canvas.width, y);
    }
    ctx.stroke();

    // Center Reference Line
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
    ctx.beginPath();
    ctx.moveTo(0, canvas.height / 2);
    ctx.lineTo(canvas.width, canvas.height / 2);
    ctx.stroke();

    if (analyserNode && activeGain) {
      analyserNode.getByteTimeDomainData(dataArray);

      // Get current theme accent color
      const computedAccent = getComputedStyle(document.documentElement).getPropertyValue('--accent-primary').trim() || '#10b981';

      ctx.lineWidth = 2.5;
      ctx.strokeStyle = computedAccent;
      ctx.shadowBlur = 10;
      ctx.shadowColor = computedAccent;

      ctx.beginPath();
      const sliceWidth = canvas.width * 1.0 / bufferLength;
      let x = 0;

      for (let i = 0; i < bufferLength; i++) {
        const v = dataArray[i] / 128.0;
        const y = v * (canvas.height / 2);

        if (i === 0) {
          ctx.moveTo(x, y);
        } else {
          ctx.lineTo(x, y);
        }
        x += sliceWidth;
      }

      ctx.lineTo(canvas.width, canvas.height / 2);
      ctx.stroke();
      ctx.shadowBlur = 0;
    } else {
      // Idle Flat Line with Soft Pulse
      const computedAccent = getComputedStyle(document.documentElement).getPropertyValue('--accent-primary').trim() || '#10b981';
      ctx.lineWidth = 1.5;
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.2)';
      ctx.beginPath();
      ctx.moveTo(0, canvas.height / 2);
      ctx.lineTo(canvas.width, canvas.height / 2);
      ctx.stroke();
    }
  }

  draw();
}

/* -------------------------------------------------------------------
   3. ARCHITECTURE PIPELINE EXPLORER
   ------------------------------------------------------------------- */
const PIPELINE_DATA = {
  1: {
    badge: 'Etapa 01 de 05 — Modelo de Dados',
    title: 'Camada de Projeto & Modelagem Temporal',
    desc: 'O projeto decompõe composições em faixas e partes independentes. As notas musicais contêm posições e durações absolutas em milissegundos calculadas em função do andamento (BPM) e da métrica de compasso. Armazena curvas contínuas de afinação (pontos de portamento, vibrato e glissando) e parâmetros dinâmicos de expressão (dynamics, velocity, modulação, sopro e gênero).',
    file: 'Rust Data Structure (src/project/model.rs)',
    code: `pub struct Note {
    pub position_ms: f64,
    pub duration_ms: f64,
    pub lyric: String,
    pub tone_number: u8, // MIDI 0..127
    pub pitch_bends: Vec<PitchPoint>,
    pub envelope: Envelope5Point,
    pub expressions: NoteExpressions,
}`
  },
  2: {
    badge: 'Etapa 02 de 05 — Análise Fonética',
    title: 'Resolução Fonética & Calibragem (oto.ini)',
    desc: 'O fonemizador converte as letras digitadas em cadeias de fonemas individuais (ataques de início de frase, transições consoante-vogal e consoantes de término). O sistema consulta a tabela oto.ini do cantor ativo e determina a amostra WAV correspondente de acordo com o tom da nota (prefix.map). Aplica preutterance para antecipar consoantes antes da batida e overlap para fusão suave.',
    file: 'Phonetic Mapping Engine (src/oto/parser.rs)',
    code: `pub struct OtoEntry {
    pub alias: String,
    pub wav_filename: String,
    pub offset: f64,        // Início útil (ms)
    pub consonant: f64,     // Consoante fixa sem stretch (ms)
    pub cutoff: f64,        // Ponto de término (ms)
    pub preutterance: f64,  // Antecipação consonantal (ms)
    pub overlap: f64,       // Zona de fusão acústica (ms)
}`
  },
  3: {
    badge: 'Etapa 03 de 05 — Motor Resampler DSP',
    title: 'Resampling, Estiramento & Transposição Tonal',
    desc: 'O motor de afinação VENUS (nativo em Rust) isola o trecho útil delimitado por offset e cutoff, preserva a consoante fixa sem deformação temporal e estica a vogal periódica através de síntese formântica pura, análise de periodicidade com algoritmo YIN e reconstrução espectral de fase mínima. Elimina artefatos anasalados e preserva harmônicos vocais.',
    file: 'Native VENUS Resampler (src/dsp/venus.rs)',
    code: `pub fn render_venus_slice(
    raw_samples: &[f32],
    oto: &OtoEntry,
    target_pitch_cents: f64,
    target_length_ms: f64,
    formant_shift: f64,
) -> Result<Vec<f32>, DspError> {
    let yin_pitch = analyze_yin_periodicity(raw_samples)?;
    let spectral_envelope = extract_minimum_phase_envelope(raw_samples)?;
    synthesize_formant_slice(spectral_envelope, target_pitch_cents, target_length_ms)
}`
  },
  4: {
    badge: 'Etapa 04 de 05 — Motor Wavtool',
    title: 'Emenda, Splicing & Envelopamento Andromeda',
    desc: 'O motor nativo Andromeda aplica a curva de amplitude UTAU de 5 pontos (p1, p2, p3, p4, p5, v1, v2, v3, v4, v5) a cada fatia e costura as amostras no buffer da faixa utilizando interpolação suave e crossfade de potência constante (equal-power crossfade), eliminando estalos de fase e transientes indesejados.',
    file: 'Andromeda Wavtool Splicer (src/renderer/track.rs)',
    code: `pub fn splice_andromeda_chunks(
    track_buffer: &mut [f32],
    slice: &[f32],
    insert_pos_samples: usize,
    overlap_samples: usize,
    envelope: &Envelope5Point,
) {
    let shaped = apply_5point_envelope(slice, envelope);
    apply_equal_power_crossfade(track_buffer, &shaped, insert_pos_samples, overlap_samples);
}`
  },
  5: {
    badge: 'Etapa 05 de 05 — Barramento DSP & Streaming',
    title: 'Streaming Multithread Rayon & Efeitos DSP',
    desc: 'O renderizador despacha tarefas em paralelo por blocos temporais (progressive chunks) através de um pool de threads gerenciado pelo Rayon. À medida que os blocos ficam prontos, eles passam pelo Rack FX (EQ de 31 bandas, Compressor, Reverb) e são enviados diretamente ao subsistema de áudio sem travamentos na interface gráfica.',
    file: 'Rayon Concurrency Pipeline (src/renderer/chunked.rs)',
    code: `pub fn render_project_parallel(
    project: &Project,
    voicebank: &Voicebank,
    output_sender: crossbeam::Sender<AudioChunk>,
) {
    project.tracks.par_iter().for_each(|track| {
        let chunk = render_track_chunk(track, voicebank);
        apply_fx_rack_inplace(&mut chunk.samples, &track.fx_settings);
        output_sender.send(chunk).ok();
    });
}`
  }
};

function initPipelineExplorer() {
  const steps = document.querySelectorAll('.pipeline-step');
  const badge = document.getElementById('stepBadge');
  const title = document.getElementById('stepTitle');
  const text = document.getElementById('stepText');
  const codeHeader = document.querySelector('#pipelineDetail .code-header span');
  const codeBlock = document.querySelector('#codeStep code');

  steps.forEach(step => {
    step.addEventListener('click', () => {
      steps.forEach(s => s.classList.remove('active'));
      step.classList.add('active');

      const stepId = parseInt(step.getAttribute('data-step'), 10);
      const data = PIPELINE_DATA[stepId];
      if (data) {
        if (badge) badge.textContent = data.badge;
        if (title) title.textContent = data.title;
        if (text) text.textContent = data.desc;
        if (codeHeader) codeHeader.textContent = data.file;
        if (codeBlock) {
          codeBlock.textContent = data.code;
        }
      }
    });
  });
}

/* -------------------------------------------------------------------
   4. THEME GALLERY TABS
   ------------------------------------------------------------------- */
function initThemeGallery() {
  const tabs = document.querySelectorAll('.gallery-tab');
  const applyBtn = document.getElementById('applySiteThemeBtn');

  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      const themeId = tab.getAttribute('data-theme-id');
      updateGalleryDisplay(themeId);
    });
  });

  if (applyBtn) {
    applyBtn.addEventListener('click', () => {
      const activeTab = document.querySelector('.gallery-tab.active');
      if (activeTab) {
        const themeId = activeTab.getAttribute('data-theme-id').replace('_', '-');
        const root = document.documentElement;
        root.setAttribute('data-theme', themeId);
        localStorage.setItem('kamafeu-theme', themeId);

        const info = THEME_DATA[themeId];
        const activeName = document.getElementById('activeThemeName');
        const activeDot = document.getElementById('activeThemeDot');
        if (activeName && info) activeName.textContent = info.name.split(' ')[0] + ' ' + info.name.split(' ')[1];
        if (activeDot && info) activeDot.className = 'theme-dot ' + info.dotClass;

        applyBtn.textContent = '✓ Tema Aplicado!';
        setTimeout(() => {
          applyBtn.textContent = 'Aplicar neste Site';
        }, 1800);
      }
    });
  }
}

function updateGalleryDisplay(themeId) {
  const themeKey = themeId.replace('_', '-');
  const data = THEME_DATA[themeKey];
  const title = document.getElementById('themeGalleryTitle');
  const desc = document.getElementById('themeGalleryDesc');
  const img = document.getElementById('themeGalleryImg');

  if (data) {
    if (title) title.textContent = `Tema ${data.name}`;
    if (desc) desc.textContent = data.desc;
    if (img) {
      img.style.opacity = '0';
      setTimeout(() => {
        img.src = data.img;
        img.style.opacity = '1';
      }, 150);
    }
  }
}

/* -------------------------------------------------------------------
   5. KEYBOARD SHORTCUTS INSTANT SEARCH
   ------------------------------------------------------------------- */
function initShortcutsSearch() {
  const input = document.getElementById('shortcutSearch');
  const items = document.querySelectorAll('.shortcut-item');

  if (input) {
    input.addEventListener('input', (e) => {
      const query = e.target.value.toLowerCase().trim();
      items.forEach(item => {
        const text = item.textContent.toLowerCase();
        item.style.display = text.includes(query) ? 'flex' : 'none';
      });
    });
  }
}

/* -------------------------------------------------------------------
   6. COMPILATION TABS ENGINE
   ------------------------------------------------------------------- */
const OS_BUILD_DATA = {
  ubuntu: {
    label: 'Ubuntu / Debian / Linux Mint / Pop!_OS',
    code: `# 1. Instalar dependências de áudio ALSA e sistema gráfico
sudo apt-get update
sudo apt-get install -y \\
  build-essential \\
  pkg-config \\
  libasound2-dev \\
  libxcb-render0-dev \\
  libxcb-shape0-dev \\
  libxcb-xfixes0-dev \\
  libxkbcommon-dev \\
  libfontconfig1-dev

# 2. Clonar o repositório
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu

# 3. Compilar em modo Release com otimização máxima
cargo build --release

# Executar o Kamafeu Studio
./target/release/kamafeu`
  },
  fedora: {
    label: 'Fedora / RHEL / CentOS Stream',
    code: `# 1. Instalar dependências no Fedora
sudo dnf install -y \\
  gcc \\
  pkg-config \\
  alsa-lib-devel \\
  libxcb-devel \\
  libxkbcommon-devel \\
  fontconfig-devel

# 2. Clonar e compilar
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu
cargo build --release

# Executar
./target/release/kamafeu`
  },
  arch: {
    label: 'Arch Linux / Manjaro / EndeavourOS',
    code: `# 1. Instalar dependências no Arch Linux
sudo pacman -S --needed \\
  base-devel \\
  pkgconf \\
  alsa-lib \\
  libxcb \\
  libxkbcommon \\
  fontconfig

# 2. Clonar e compilar
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu
cargo build --release

# Executar
./target/release/kamafeu`
  },
  freebsd: {
    label: 'FreeBSD (Clang & pkgconf)',
    code: `# 1. Instalar dependências via pkg
sudo pkg install -y \\
  rust \\
  pkgconf \\
  alsa-lib \\
  libxcb \\
  libxkbcommon \\
  fontconfig

# 2. Clonar e compilar
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu
cargo build --release

# Executar
./target/release/kamafeu`
  },
  macos: {
    label: 'macOS (Apple Silicon M1/M2/M3/M4 & Intel x86_64)',
    code: `# 1. Instalar Xcode Command Line Tools (CoreAudio e Metal são nativos)
xcode-select --install

# 2. Clonar o repositório
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu

# 3. Compilar com aceleração nativa Metal/Cocoa
cargo build --release

# Executar o binário do Kamafeu
./target/release/kamafeu`
  },
  windows: {
    label: 'Windows 10/11 (MSVC C++ Toolchain)',
    code: `# 1. Instale o Visual Studio Build Tools com "Desktop C++"
# 2. No terminal PowerShell:
git clone https://github.com/studiopomar/kamafeu.git
cd kamafeu

# 3. Compilar em modo release
cargo build --release

# Executar
.\\target\\release\\kamafeu.exe`
  },
  android: {
    label: 'Android (Cross-compilation APK via cargo-apk)',
    code: `# 1. Instalar cargo-apk e target ARM64
cargo install cargo-apk
rustup target add aarch64-linux-android

# 2. Definir ANDROID_NDK_ROOT e compilar o APK
cargo apk build --lib --release --target aarch64-linux-android

# O APK final é gerado em:
# target/release/apk/kamafeu.apk`
  }
};

function initCompilationTabs() {
  const tabs = document.querySelectorAll('.os-tab');
  const label = document.getElementById('osCodeLabel');
  const codeBlock = document.querySelector('#osCodeBlock code');

  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');

      const osKey = tab.getAttribute('data-os');
      const data = OS_BUILD_DATA[osKey];
      if (data) {
        if (label) label.textContent = data.label;
        if (codeBlock) codeBlock.textContent = data.code;
      }
    });
  });
}

/* -------------------------------------------------------------------
   7. COPY TO CLIPBOARD BUTTONS
   ------------------------------------------------------------------- */
function initCopyButtons() {
  const copyBtns = document.querySelectorAll('.btn-copy');

  copyBtns.forEach(btn => {
    btn.addEventListener('click', () => {
      const targetId = btn.getAttribute('data-target');
      const targetEl = document.getElementById(targetId);
      if (targetEl) {
        const textToCopy = targetEl.textContent;
        navigator.clipboard.writeText(textToCopy).then(() => {
          const originalText = btn.textContent;
          btn.textContent = '✓ Copiado!';
          btn.style.background = 'var(--accent-primary)';
          btn.style.color = 'var(--text-on-accent)';

          setTimeout(() => {
            btn.textContent = originalText;
            btn.style.background = '';
            btn.style.color = '';
          }, 2000);
        }).catch(err => {
          console.error('Falha ao copiar:', err);
        });
      }
    });
  });
}

/* -------------------------------------------------------------------
   8. MOBILE NAVIGATION MENU TOGGLE
   ------------------------------------------------------------------- */
function initMobileMenu() {
  const toggle = document.getElementById('mobileToggle');
  const nav = document.getElementById('navMenu');

  if (toggle && nav) {
    toggle.addEventListener('click', () => {
      nav.classList.toggle('show');
    });

    nav.querySelectorAll('.nav-link').forEach(link => {
      link.addEventListener('click', () => {
        nav.classList.remove('show');
      });
    });
  }
}
