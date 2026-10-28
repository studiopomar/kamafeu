# Descrições dos arquivos desta atualização

Este índice explica em português a função de cada arquivo alterado ou adicionado nesta atualização. Os caminhos são relativos à raiz do repositório.

| Arquivo | Descrição |
| --- | --- |
| `.github/workflows/build_release.yml` | Define a compilação e o empacotamento dos binários de lançamento para as plataformas suportadas. |
| `.github/workflows/ci.yml` | Define as verificações automatizadas de qualidade, compilação e testes do projeto. |
| `.github/workflows/package_linux.yml` | Define a geração de pacotes Linux e seus componentes nativos. |
| `.gitignore` | Lista arquivos e diretórios locais que o Git deve ignorar. |
| `Cargo.lock` | Registra as versões exatas das dependências Rust usadas na compilação. |
| `Cargo.toml` | Declara os metadados, dependências, recursos e alvos do projeto Rust. |
| `NOTICE.md` | Reúne créditos, avisos de licença e informações de proveniência de componentes. |
| `README.md` | Apresenta o Kamafeu Studio, suas capacidades e orientações iniciais de uso. |
| `assets/github-social-preview.png` | Imagem de apresentação exibida ao compartilhar o repositório em redes sociais. |
| `assets/kamuse/cvc-units.svg` | Ilustração vetorial das unidades CVC usadas na autoria KAMUSE. |
| `assets/kamuse/empty.svg` | Ilustração vetorial do estado vazio do editor KAMUSE. |
| `assets/kamuse/import.svg` | Ilustração vetorial que representa a importação de gravações no KAMUSE. |
| `assets/kamuse/mark.svg` | Ilustração vetorial da etapa de marcação de unidades no KAMUSE. |
| `assets/kamuse/play.svg` | Ilustração vetorial do controle de reprodução do KAMUSE. |
| `assets/visual-prototypes/empty-project.svg` | Protótipo vetorial da tela inicial sem projeto aberto. |
| `assets/visual-prototypes/opening.svg` | Protótipo vetorial do estado de abertura de um projeto. |
| `assets/visual-prototypes/playback.svg` | Protótipo vetorial do estado de reprodução no editor. |
| `assets/visual-prototypes/rendering.svg` | Protótipo vetorial do estado de renderização de áudio. |
| `docs/gui-framework-and-visual-system.md` | Documenta a arquitetura da interface gráfica e as regras do sistema visual. |
| `docs/kfv2/engine-audit.md` | Registra a auditoria de engines e resamplers no formato de projeto KFV2. |
| `docs/kfv2/tainah.md` | Documenta a integração e o comportamento do motor vocal Tainah em projetos KFV2. |
| `native/console_tui/CMakeLists.txt` | Configura a compilação CMake do painel de terminal nativo. |
| `native/console_tui/main.cpp` | Implementa a interface de terminal do Kamafeu com FTXUI. |
| `native/console_tui/smoke.py` | Verifica a inicialização e o transporte básico da interface de terminal. |
| `native/tainah/CMakeLists.txt` | Configura a compilação da biblioteca nativa Tainah. |
| `native/tainah/Tainah.yaml` | Define os metadados e as opções de compilação do projeto Tainah. |
| `native/tainah/build.py` | Compila e empacota a biblioteca Tainah para uso pelo Kamafeu. |
| `native/tainah/patches/LOG.h` | Fornece a camada de compatibilidade de registro de mensagens usada na compilação nativa. |
| `native/tainah/patches/build-timing.patch` | Adapta a compilação da Tainah para expor informações de temporização. |
| `native/tainah/tainah.cpp` | Implementa as funções nativas de síntese e processamento WORLD da Tainah. |
| `native/tainah/tainah.h` | Declara a interface C da biblioteca Tainah consumida pelo Kamafeu. |
| `native/tainah/tainah_cli.cpp` | Implementa comandos de terminal para diagnóstico e renderização com a Tainah. |
| `native/tainah/tainah_cli_test.py` | Verifica os comandos e resultados esperados da interface de terminal Tainah. |
| `native/tainah/tainah_timing.h` | Declara as estruturas e funções de medição de temporização do motor. |
| `native/tainah/tainah_timing_test.cpp` | Verifica o comportamento das medições de temporização da Tainah. |
| `packaging/flatpak/org.studiopomar.Kamafeu.yml` | Descreve a construção do aplicativo Kamafeu no formato Flatpak. |
| `resamplers/README.md` | Explica a instalação, organização e uso dos resamplers externos. |
| `scripts/release-smoke.py` | Faz uma verificação rápida de inicialização do aplicativo empacotado. |
| `site/assets/editor-capture.png` | Captura visual do editor usada na página pública do Kamafeu. |
| `site/assets/hero-sound.svg` | Ilustração vetorial principal da página pública do Kamafeu. |
| `site/index.html` | Estrutura e conteúdo da página inicial pública do projeto. |
| `site/script.js` | Implementa as interações e animações da página pública. |
| `site/style.css` | Define o layout responsivo e a identidade visual da página pública. |
| `src/audio/fx/mod.rs` | Organiza os módulos de efeitos de áudio. |
| `src/audio/fx/processor.rs` | Processa o sinal de áudio através da cadeia de efeitos. |
| `src/audio/fx/settings.rs` | Define parâmetros e configurações dos efeitos de áudio. |
| `src/audio/master_output.rs` | Gerencia a saída master e o processamento final do áudio. |
| `src/audio/mod.rs` | Declara e organiza os componentes de áudio do aplicativo. |
| `src/audio/player.rs` | Controla reprodução, interrupção e sincronização do áudio. |
| `src/config.rs` | Armazena e carrega as preferências e configurações do Kamafeu. |
| `src/copaiba/gui.rs` | Implementa elementos gráficos do ambiente de autoria Copaiba. |
| `src/copaiba/gui/kamuse_ui.rs` | Implementa a interface de edição e gravação do KAMUSE. |
| `src/copaiba/kamuse.rs` | Define os dados e operações do fluxo de autoria KAMUSE. |
| `src/copaiba/mod.rs` | Organiza os módulos e pontos de entrada do Copaiba. |
| `src/drivers/resampler_driver.rs` | Integra drivers de resampling e normaliza suas opções de execução. |
| `src/drivers/tainah_ffi.rs` | Faz a ponte FFI entre o código Rust e a biblioteca Tainah. |
| `src/dsp/world_resampler.rs` | Adapta o processamento de resampling baseado em WORLD. |
| `src/formats/aps.rs` | Lê e grava projetos no formato APS. |
| `src/gui/animation.rs` | Gerencia animações e transições da interface gráfica. |
| `src/gui/app_frame.rs` | Monta o quadro principal e a estrutura visual do aplicativo. |
| `src/gui/arrangement.rs` | Implementa a organização temporal e a visualização de arranjo. |
| `src/gui/arrangement/vertical.rs` | Implementa a disposição vertical dos elementos do arranjo. |
| `src/gui/audio_export.rs` | Conduz a exportação de áudio dos projetos. |
| `src/gui/background_tasks.rs` | Coordena tarefas demoradas em segundo plano e seus estados visuais. |
| `src/gui/dialogs/copaiba.rs` | Implementa diálogos relacionados às ferramentas Copaiba. |
| `src/gui/dialogs/fx_rack_dialog.rs` | Apresenta a janela de configuração da cadeia de efeitos. |
| `src/gui/dialogs/project_diagnostics.rs` | Apresenta diagnósticos e verificações do projeto aberto. |
| `src/gui/dialogs/singers_gallery.rs` | Exibe a galeria de cantores e bancos de voz disponíveis. |
| `src/gui/editor_canvas.rs` | Desenha a área central de edição musical. |
| `src/gui/editor_panels.rs` | Organiza os painéis acopláveis e auxiliares do editor. |
| `src/gui/engine_install.rs` | Orienta a instalação e localização de engines e resamplers. |
| `src/gui/fx_rack_ui.rs` | Desenha os controles da cadeia de efeitos de áudio. |
| `src/gui/marquee.rs` | Implementa seleção retangular de itens na interface. |
| `src/gui/menu_bar/view.rs` | Desenha os comandos e opções do menu de visualização. |
| `src/gui/mod.rs` | Declara e organiza os módulos da interface gráfica. |
| `src/gui/phoneme_palette.rs` | Apresenta ferramentas e opções de edição de fonemas. |
| `src/gui/phoneme_ruler.rs` | Desenha e permite editar os limites temporais dos fonemas. |
| `src/gui/piano_roll/interaction_tests.rs` | Contém verificações das interações do piano roll. |
| `src/gui/piano_roll/minimap.rs` | Desenha a minimapa de navegação do piano roll. |
| `src/gui/piano_roll/mod.rs` | Organiza o editor de notas e os componentes do piano roll. |
| `src/gui/piano_roll/navigation.rs` | Controla navegação, zoom e rolagem do piano roll. |
| `src/gui/piano_roll/phoneme_cache.rs` | Mantém em cache os dados de fonemas usados pelo piano roll. |
| `src/gui/piano_roll/processed_waveform.rs` | Desenha a forma de onda processada associada às notas. |
| `src/gui/piano_roll/state.rs` | Armazena o estado de interação e visualização do piano roll. |
| `src/gui/playback.rs` | Liga os controles visuais ao mecanismo de reprodução. |
| `src/gui/project_files.rs` | Implementa operações gráficas de abertura, salvamento e organização de projetos. |
| `src/gui/startup.rs` | Monta a experiência inicial e a restauração de sessão do aplicativo. |
| `src/gui/theme.rs` | Define cores, estilos e elementos visuais compartilhados. |
| `src/gui/toolbar.rs` | Desenha a barra de ferramentas e seus comandos de edição. |
| `src/gui/types.rs` | Declara tipos de dados compartilhados pela interface gráfica. |
| `src/gui/unified_panel.rs` | Organiza o painel unificado de propriedades e ferramentas. |
| `src/gui/unified_panel/engine.rs` | Apresenta configurações e informações do motor de síntese. |
| `src/gui/unified_panel/note.rs` | Apresenta propriedades e controles da nota selecionada. |
| `src/gui/unified_panel/singer.rs` | Apresenta configurações do cantor, voz e banco selecionados. |
| `src/gui/visual_fx.rs` | Implementa efeitos visuais e respostas gráficas da interface. |
| `src/gui/voicebank_actions.rs` | Implementa ações de gerenciamento de bancos de voz na interface. |
| `src/main.rs` | Inicializa o aplicativo e conecta seus componentes principais. |
| `src/oto/singers.rs` | Interpreta configurações oto e metadados de cantores. |
| `src/project/diff.rs` | Calcula diferenças entre estados de um projeto. |
| `src/project/model.rs` | Define o modelo de dados e as entidades persistidas do projeto. |
| `src/renderer/project.rs` | Prepara e renderiza as partes vocais de um projeto. |
| `src/terminal_log.rs` | Organiza o registro de mensagens para o terminal. |
| `src/terminal_log/tui.rs` | Encaminha mensagens e eventos de registro para a interface de terminal. |
| `src/voicebank/cantor.rs` | Implementa a leitura e execução de cantores nativos do Kamafeu. |
| `src/voicebank/diffsinger.rs` | Carrega e interpreta bancos de voz no formato DiffSinger. |
| `src/voicebank/diffsinger_render.rs` | Executa a renderização ONNX de bancos DiffSinger. |
| `src/voicebank/mod.rs` | Organiza as implementações de bancos de voz disponíveis. |
| `src/voicebank/native.rs` | Carrega bancos de voz nativos e suas configurações. |
| `src/voicebank/project.rs` | Conecta bancos de voz às necessidades de um projeto. |
| `src/voicebank/render.rs` | Coordena a renderização de notas usando bancos de voz. |
| `docs/indice-arquivos.md` | Explica em português a finalidade dos arquivos desta atualização. |
