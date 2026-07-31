# CHANGELOG - Kamafeu Synthesizer

Todas as mudanças notáveis e novas funcionalidades do projeto **Kamafeu** serão documentadas neste arquivo.

O formato segue a estrutura [Keep a Changelog](https://keepachangelog.com/pt-BR/1.0.0/) e este projeto adere ao versionamento semântico [Semantic Versioning](https://semver.org/lang/pt-BR/).

---

## [Unreleased] - v0.2.0-unstable

### Copaiba Voicebank Toolkit
- **Novo Formato `copaiba.config` (JSON)**: Estrutura JSON moderna para configuração de voicebanks com suporte a:
  - Corte Inicial (`corte_inicial_ms`)
  - Início da Consoante (`consoante_ms`)
  - Região de Loop (`loop_inicio_ms` e `loop_fim_ms`)
  - Cauda Final para ditongos (`cauda_final_ms`)
  - Corte Final (`corte_final_ms`)
- **Visualizador Triplo de Formas de Onda (*Stacked 3-Waveform View*)**:
  - **Forma de Onda Anterior (▲)**: Prévia do alias anterior para alinhamento contínuo.
  - **Forma de Onda Ativa (Central)**: Editor gráfico de alta legibilidade com envelopes de pico min/max e réguas em milissegundos.
  - **Forma de Onda Próxima (▼)**: Prévia do próximo alias na lista.
- **Interatividade Completa dos Marcadores**: Manipulação por clique e arraste com o mouse para todos os 5 marcadores (incluindo a Cauda Final).
- **Lista e Duplicação de Alias**: Suporte a múltiplos aliases por arquivo `.wav`, filtro de busca por texto e botão de duplicação com 1 clique (`+ Duplicar Alias`).
- **Executável Independente**: Binário dedicado (`cargo run --bin copaiba`) e integração direta na barra de ferramentas do Kamafeu Studio.

### Configuração Persistente & Histórico de Voicebanks
- **Abertura Automática do Último Voicebank**: Salva as preferências em `~/.config/kamafeu/kamafeu_config.json` e carrega automaticamente o último cantor utilizado ao iniciar o app.
- **Menu de Recentes (*Quick-Select*)**: Lista suspensa `Recentes...` no painel esquerdo para alternar entre voicebanks anteriores com 1 clique.

### Arranjo & Linha do Tempo (Arrangement View)
- **Rolagem Vertical para Múltiplas Faixas**: Container responsivo com rolagem vertical suave para navegação em projetos com dezenas de faixas.
- **Mini-Mapa Sincronizado**: O cursor de reprodução vermelho (*Playhead*) é desenhado em tempo real na visão de arranjo e sincronizado com o Piano Roll.
- **Clipes de Áudio/Voz (*UVoicePart*)**: Arraste e solte (*drag & drop*) de partes pela linha do tempo e foco por duplo clique.

### Interface do Usuário (UI) & Localização
- **Tradução Completa para Português**: Interface 100% traduzida em todas as ferramentas, painéis e controles.
- **Estética Profissional sem Emojis**: Remoção de emojis da interface para um visual elegante de DAW.
- **Avatar do Cantor sem Distorção**: Carregamento da imagem do cantor (`character.txt` / `character.yaml`) centralizado em caixa 100x100 px com taxa de proporção (*aspect ratio*) preservada.
- **Correção de Contraste**: Tema de alto contraste para botões de seleção de taxa de amostragem (44100 Hz / 48000 Hz) e menus suspensos sobre fundo roxo escuro.

### Motor de Síntese & CI/CD
- **Pipeline de Releases Automatizado**: Workflow do GitHub Actions configurado para compilar artefatos `.dmg` (macOS) e `.apk` (Android) etiquetados como `unstable`.
- **Suporte a Motores Externos e Nativos**: Integração de resamplers nativos Rust (TD-PSOLA/SOLA) e drivers externos (`macres`, `wavtool-yawu`).
- **Formatos de Projeto**: Importação e exportação de projetos `.ust` (UTAU) e `.ustx` (OpenUTAU).

---

## [1.0.0-alpha.1] - 2026-07-30

- Versão Alpha inicial do Kamafeu Synthesizer Core.
- Suporte a leitura de Voicebanks UTAU (`oto.ini`, `prefix.map`).
- Engine gráfica desenvolvida em Rust com `eframe` / `egui`.
