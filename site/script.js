// Tema global do site (Outono, Dark, Claro)
function initThemeSwitcher() {
  const themeButtons = document.querySelectorAll('.theme-toggle-btn');
  
  // Recuperar preferencia salva ou padrao 'theme-rosa'
  const savedTheme = localStorage.getItem('kamafeu-theme') || 'theme-rosa';
  applyTheme(savedTheme);

  themeButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      const theme = btn.getAttribute('data-theme');
      applyTheme(theme);
      localStorage.setItem('kamafeu-theme', theme);
    });
  });
}

function applyTheme(themeName) {
  document.body.className = themeName;

  const themeButtons = document.querySelectorAll('.theme-toggle-btn');
  themeButtons.forEach(btn => {
    if (btn.getAttribute('data-theme') === themeName) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  });
}

// Visualizador de temas da aplicação Kamafeu com efeito de fade no site inteiro
function showTheme(themeId) {
  const tabs = document.querySelectorAll('.tab');
  tabs.forEach(tab => tab.classList.remove('active'));

  const panels = document.querySelectorAll('.theme-panel');
  panels.forEach(panel => panel.classList.remove('active'));

  const activePanel = document.getElementById(`theme-${themeId}`);
  if (activePanel) {
    activePanel.classList.add('active');
  }

  const activeTab = Array.from(tabs).find(tab => tab.getAttribute('onclick')?.includes(themeId));
  if (activeTab) {
    activeTab.classList.add('active');
  }

  // Aplica o tema visual no site inteiro com transicao suave (fade)
  const fullThemeName = `theme-${themeId}`;
  document.body.className = fullThemeName;

  // Atualiza botoes do header caso exista correspondente
  const themeButtons = document.querySelectorAll('.theme-toggle-btn');
  themeButtons.forEach(btn => {
    if (btn.getAttribute('data-theme') === fullThemeName) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  });
}

// Paralaxe e dinamica suave do banner
function initBannerParallax() {
  const banner = document.querySelector('.hero-logo-banner');
  const heroSection = document.querySelector('.hero');
  if (!banner || !heroSection) return;

  // Paralaxe sutil ao mover o mouse na tela
  window.addEventListener('mousemove', (e) => {
    const xRatio = (e.clientX / window.innerWidth - 0.5) * 2;
    const yRatio = (e.clientY / window.innerHeight - 0.5) * 2;

    const moveX = xRatio * 10;
    const moveY = yRatio * 6;
    const tiltX = -yRatio * 4;
    const tiltY = xRatio * 4;

    banner.style.transform = `translate(${moveX}px, ${moveY}px) rotateX(${tiltX}deg) rotateY(${tiltY}deg)`;
  });

  // Reset suave quando o mouse sai da janela
  document.addEventListener('mouseleave', () => {
    banner.style.transform = '';
  });
}

document.addEventListener('DOMContentLoaded', () => {
  initThemeSwitcher();
  initBannerParallax();
});
