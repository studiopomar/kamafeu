/* ===================================================================
   KAMAFEU STUDIO - WORKSTATION PORTAL JAVASCRIPT
   Clean, authentic interactive logic
   =================================================================== */

document.addEventListener('DOMContentLoaded', () => {
  initOsDetection();
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
   2. INTERACTIVE OTO.INI WAVEFORM CALIBRATOR (CANVAS)
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
  const statusEl = document.getElementById('calibratorStatus');

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
    const totalMs = 600; // total viewport ms

    ctx.fillStyle = '#06080c';
    ctx.fillRect(0, 0, w, h);

    // Coordinate conversion
    const msToX = (ms) => (ms / totalMs) * w;

    const xOffset = msToX(offsetMs);
    const xConsonant = msToX(consonantMs);
    const xPreut = msToX(preutMs);
    const xOverlap = msToX(offsetMs + overlapMs);
    const xCutoff = msToX(totalMs + cutoffMs);

    // 1. Draw Offset Blanking Zone (Left cut)
    ctx.fillStyle = 'rgba(239, 68, 68, 0.15)';
    ctx.fillRect(0, 0, xOffset, h);

    // 2. Draw Consonant Fixed Zone (Unstretched pink)
    ctx.fillStyle = 'rgba(245, 158, 11, 0.18)';
    ctx.fillRect(xOffset, 0, Math.max(0, xConsonant - xOffset), h);

    // 3. Draw Overlap Blend Zone
    ctx.fillStyle = 'rgba(139, 92, 246, 0.18)';
    ctx.fillRect(xOffset, 0, Math.max(0, xOverlap - xOffset), h);

    // 4. Draw Cutoff Blanking Zone (Right cut)
    ctx.fillStyle = 'rgba(107, 114, 128, 0.25)';
    ctx.fillRect(xCutoff, 0, Math.max(0, w - xCutoff), h);

    // Grid center line
    ctx.strokeStyle = '#1e2430';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();

    // Draw Realistic Vocal Waveform (Consonant attack noise + Harmonic vowel periodic wave)
    ctx.strokeStyle = '#34d399';
    ctx.lineWidth = 1.5;
    ctx.beginPath();

    const midY = h / 2;
    for (let x = 0; x < w; x++) {
      const ms = (x / w) * totalMs;
      let amp = 0;

      if (ms < offsetMs || ms > (totalMs + cutoffMs)) {
        amp = (Math.random() - 0.5) * 4; // silence / noise floor
      } else if (ms < consonantMs) {
        // Consonant transient "K" burst + noise
        const env = Math.sin(((ms - offsetMs) / (consonantMs - offsetMs)) * Math.PI);
        const noise = (Math.random() - 0.5) * 45;
        const transient = Math.sin(ms * 0.4) * 35;
        amp = (noise + transient) * env;
      } else {
        // Periodic Vowel "A" waveform (Fundamental F0 + Formants F1/F2)
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

    // Draw Vertical Parameter Marker Lines
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
   3. THEME GALLERY TABS
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
   4. SOURCE COMPILATION TABS & COPY
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
   5. MOBILE NAV MENU
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
