/**
 * Kamafeu Studio - Interações da Landing Page
 * Recursos: Visualizador Espectral em Tempo Real, Detecção de Sistema Operacional,
 * Troca de Temas da Interface e Menu Mobile.
 */

(function () {
  "use strict";

  // --- 1. Detecção do Sistema Operacional ---
  function detectarSistema() {
    const ua = navigator.userAgent.toLowerCase();
    const platform = (navigator.userAgentData?.platform || navigator.platform || "").toLowerCase();

    if (platform.includes("mac") || ua.includes("macintosh") || ua.includes("mac os x")) {
      return "mac";
    } else if (platform.includes("win") || ua.includes("windows")) {
      return "win";
    } else if (platform.includes("linux") || ua.includes("linux") || ua.includes("x11")) {
      return "linux";
    }
    return "outro";
  }

  function aplicarDestaqueSistema() {
    const so = detectarSistema();
    const textoBotao = document.getElementById("heroDownloadText");

    if (so === "mac") {
      if (textoBotao) textoBotao.textContent = "Baixar para macOS (Universal)";
      const card = document.getElementById("cardMac");
      const badge = document.getElementById("badgeMac");
      if (card) card.classList.add("highlight");
      if (badge) badge.style.display = "inline-block";
    } else if (so === "win") {
      if (textoBotao) textoBotao.textContent = "Baixar para Windows (x64)";
      const card = document.getElementById("cardWin");
      const badge = document.getElementById("badgeWin");
      if (card) card.classList.add("highlight");
      if (badge) badge.style.display = "inline-block";
    } else if (so === "linux") {
      if (textoBotao) textoBotao.textContent = "Baixar para Linux (x86_64)";
      const card = document.getElementById("cardLinux");
      const badge = document.getElementById("badgeLinux");
      if (card) card.classList.add("highlight");
      if (badge) badge.style.display = "inline-block";
    } else {
      if (textoBotao) textoBotao.textContent = "Baixar Kamafeu 3.0";
    }
  }

  // --- 2. Galeria de Temas ---
  const imagensTemas = {
    "pomar": "assets/themes/theme_pomar_neon.png",
    "cyberpunk": "assets/themes/theme_cyberpunk.png",
    "melodyne": "assets/themes/theme_melodyne_gold.png",
    "mikan": "assets/themes/theme_mikan_teal.png",
    "nordic": "assets/themes/theme_nordic_slate.png"
  };

  function inicializarTemas() {
    const botoes = document.querySelectorAll(".theme-pill-btn");
    const previsualizacao = document.getElementById("themePreviewImage");
    if (!previsualizacao) return;

    botoes.forEach(function (btn) {
      btn.addEventListener("click", function () {
        const tema = btn.getAttribute("data-theme");
        if (tema && imagensTemas[tema]) {
          botoes.forEach(function (b) {
            b.classList.remove("active");
            b.setAttribute("aria-selected", "false");
          });
          btn.classList.add("active");
          btn.setAttribute("aria-selected", "true");

          previsualizacao.style.opacity = "0.2";
          setTimeout(function () {
            previsualizacao.src = imagensTemas[tema];
            previsualizacao.style.opacity = "1";
          }, 150);
        }
      });
    });
  }

  // --- 3. Visualizador de Áudio em Tempo Real (Canvas) ---
  function inicializarVisualizadorAudio() {
    const canvas = document.getElementById("audioVisualizerCanvas");
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const prefereReducaoMovimento = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    let largura = (canvas.width = canvas.parentElement.clientWidth || 1080);
    let altura = (canvas.height = 70);

    window.addEventListener("resize", function () {
      largura = canvas.width = canvas.parentElement.clientWidth || 1080;
      altura = canvas.height = 70;
      if (prefereReducaoMovimento) desenharOndaEstatica();
    });

    const totalBarras = 64;
    const frequencias = new Float32Array(totalBarras);
    for (let i = 0; i < totalBarras; i++) {
      frequencias[i] = Math.sin((i / totalBarras) * Math.PI) * 0.8;
    }

    let fase = 0;

    function desenharOndaEstatica() {
      ctx.clearRect(0, 0, largura, altura);
      ctx.fillStyle = "rgba(192, 132, 252, 0.4)";
      const larguraBarra = largura / totalBarras;
      for (let i = 0; i < totalBarras; i++) {
        const alturaBarra = frequencias[i] * altura * 0.7;
        const x = i * larguraBarra;
        const y = (altura - alturaBarra) / 2;
        ctx.fillRect(x + 1, y, Math.max(1, larguraBarra - 2), alturaBarra);
      }
    }

    if (prefereReducaoMovimento) {
      desenharOndaEstatica();
      return;
    }

    function animarQuadro() {
      fase += 0.045;
      ctx.clearRect(0, 0, largura, altura);

      const larguraBarra = largura / totalBarras;

      for (let i = 0; i < totalBarras; i++) {
        const base = Math.sin((i / totalBarras) * Math.PI);
        const osc1 = Math.sin(fase * 1.5 + i * 0.28) * 0.35;
        const osc2 = Math.cos(fase * 2.1 + i * 0.15) * 0.25;
        const osc3 = Math.sin(fase * 0.7 - i * 0.1) * 0.15;

        const amp = Math.max(0.08, base * (0.6 + osc1 + osc2 + osc3));
        const alturaBarra = amp * (altura * 0.85);

        const x = i * larguraBarra;
        const y = (altura - alturaBarra) / 2;

        const proporcao = i / totalBarras;
        const gradiente = ctx.createLinearGradient(x, y, x, y + alturaBarra);
        if (proporcao < 0.5) {
          gradiente.addColorStop(0, "rgba(244, 63, 94, 0.85)");
          gradiente.addColorStop(1, "rgba(192, 132, 252, 0.4)");
        } else {
          gradiente.addColorStop(0, "rgba(192, 132, 252, 0.85)");
          gradiente.addColorStop(1, "rgba(56, 189, 248, 0.45)");
        }

        ctx.fillStyle = gradiente;
        ctx.fillRect(x + 1, y, Math.max(1, larguraBarra - 2), alturaBarra);

        // Nós de afinação
        if (i % 8 === 0) {
          ctx.fillStyle = "#ffffff";
          ctx.beginPath();
          ctx.arc(x + larguraBarra / 2, y + alturaBarra / 2, 2, 0, Math.PI * 2);
          ctx.fill();
        }
      }

      requestAnimationFrame(animarQuadro);
    }

    requestAnimationFrame(animarQuadro);
  }

  // --- 4. Menu Responsivo Mobile ---
  function inicializarMenuMobile() {
    const botao = document.getElementById("mobileMenuBtn");
    const nav = document.querySelector(".nav-links");
    if (!botao || !nav) return;

    botao.addEventListener("click", function () {
      const expandido = botao.getAttribute("aria-expanded") === "true";
      botao.setAttribute("aria-expanded", String(!expandido));
      if (!expandido) {
        nav.style.display = "flex";
        nav.style.flexDirection = "column";
        nav.style.position = "absolute";
        nav.style.top = "4.25rem";
        nav.style.left = "0";
        nav.style.width = "100%";
        nav.style.background = "#09080d";
        nav.style.padding = "1.5rem";
        nav.style.borderBottom = "1px solid rgba(255,255,255,0.1)";
        nav.style.zIndex = "99";
      } else {
        nav.style.display = "";
      }
    });

    // Fechar ao clicar em um link
    nav.querySelectorAll("a").forEach(function (link) {
      link.addEventListener("click", function () {
        if (window.innerWidth <= 900) {
          nav.style.display = "";
          botao.setAttribute("aria-expanded", "false");
        }
      });
    });
  }

  // --- Inicialização ao Carregar o DOM ---
  document.addEventListener("DOMContentLoaded", function () {
    aplicarDestaqueSistema();
    inicializarTemas();
    inicializarVisualizadorAudio();
    inicializarMenuMobile();
  });
})();
