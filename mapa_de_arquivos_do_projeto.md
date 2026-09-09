# Mapa de Arquivos do Projeto Kamafeu Studio

Este documento serve como referência e índice técnico completo de todos os arquivos e diretórios do repositório **Kamafeu Studio**. Utilize este mapa para localizar rapidamente módulos, estruturas de dados, pipelines de áudio, conversores de formato e componentes de interface gráfica sem necessidade de inspecionar o código-fonte manualmente.
- xiao do Studio Pomar

---

## Estrutura da Raiz

| Arquivo / Pasta | Finalidade |
| --- | --- |
| `Cargo.toml` | Manifesto principal do Rust: dependências, metadados do pacote e configurações do Android NDK (`cargo-apk`). |
| `Cargo.lock` | Registro de versões exatas e árvores de dependências resolvidas do Cargo. |
| `build.rs` | Script de compilação para geração de recursos do sistema e ícones nativos. |
| `CHANGELOG.md` | Histórico cronológico de lançamentos, novas funcionalidades, melhorias e correções. |
| `LICENSE` | Licença MIT sob a qual o Kamafeu Studio é distribuído. |
| `README.md` | Apresentação do projeto, guia de primeiros passos, arquitetura, compilação multi-SO e atalhos. |
| `mapa_de_arquivos_do_projeto.md` | Índice técnico completo de arquivos e suas atribuições arquiteturais no código. |
| `assets/` | Recursos estáticos: ícones do sistema, imagens de interface e fontes TTF (`Outfit`). |
| `docs/` | Guias de desenvolvimento, documentação técnica de assinatura de APKs e notas de arquitetura. |
| `res/` | Recursos específicos de empacotamento do Android (`res/mipmap` e manifestos). |
| `resamplers/` | Binários de resamplers externos (`straycat-rs`, `macres`, `moresampler`, etc.) e documentação de licenças. |
| `wavtools/` | Utilitários de linha de comando para junção e emenda de fonemas (`wavtool-yawu`, `wavtool`). |
| `tests/` | Suíte de testes de integração ponta a ponta e testes de formatos. |
| `copaiba-neo/` | Protótipos e utilitários auxiliares da ferramenta Copaiba. |
| `src/` | Código-fonte principal da aplicação em Rust. |

---

## Módulos Raiz em `src/`

| Arquivo | Descrição |
| --- | --- |
| `src/main.rs` | Ponto de entrada do executável `kamafeu`. Processa argumentos de linha de comando (CLI), despacho de comandos em lote (`render`, `voicebank-info`) e inicialização da janela gráfica via `eframe`. |
| `src/lib.rs` | Biblioteca central (`kamafeu`). Reexporta módulos públicos, tipos fundamentais e pontos de integração para builds de desktop e Android (`cdylib`). |
| `src/config.rs` | Gerenciamento de configurações e preferências do usuário (idioma, tema, resampler/wavtool selecionados, buffers de áudio, caminhos de voicebank e persistência JSON em disco). |
| `src/discord_rpc.rs` | Integração com Discord Rich Presence: envia status de edição, projeto atual, cantor selecionado e tempo decorrido para o perfil do usuário no Discord. |
| `src/copaiba_bridge.rs` | Ponte de sincronização entre o editor principal e o utilitário Copaiba para edição de `oto.ini` em tempo real. |
| `src/dialogs.rs` | Interface unificada de despacho de diálogos modais e mensagens de alerta da interface gráfica. |

---

## `src/bin/` — Binários Autônomos

| Arquivo | Descrição |
| --- | --- |
| `src/bin/copaiba.rs` | Ponto de entrada do utilitário independente **Copaiba Toolkit** para calibração, inspeção de formas de onda e organização de bancos de voz UTAU. |

---

## `src/audio/` — Motor de Reprodução e Efeitos (DSP FX)

| Arquivo / Subpasta | Descrição |
| --- | --- |
| `src/audio/mod.rs` | Módulo raiz do subsistema de áudio. Exporta carregadores, players e rack de efeitos. |
| `src/audio/player.rs` | Implementação do reprodutor em tempo real (`AudioPlayer`) utilizando `rodio` / `cpal`. Gerencia stream de áudio, controle de posição (seek), volume mestre, playhead e envio contínuo de buffers. |
| `src/audio/loader.rs` | Utilitários de decodificação e carregamento de arquivos WAV e FLAC para memória em formato de ponto flutuante (`Vec<f32>`). |
| `src/audio/metronome.rs` | Gerador e sintetizador do metrônomo de cliques, com acentuação tonal no primeiro tempo do compasso e cálculo baseado em BPM. |
| `src/audio/fx/mod.rs` | Módulo raiz do rack de efeitos estéreo aplicáveis por faixa de áudio. |
| `src/audio/fx/processor.rs` | Processador unificado do canal de efeitos. Encadeia equalizador, compressor, chorus, delay e reverb. |
| `src/audio/fx/settings.rs` | Estruturas de dados de configuração dos efeitos (ganhos, frequências de corte, thresholds, decay, wet/dry). |
| `src/audio/fx/equalizer.rs` | Equalizador gráfico de 31 bandas em terças de oitava e equalizador paramétrico de precisão. |
| `src/audio/fx/biquad.rs` | Filtros IIR digitais biquad (Lowpass, Highpass, Bandpass, Peaking, HighShelf, LowShelf). |
| `src/audio/fx/tests.rs` | Testes unitários do processamento de áudio dos filtros e processadores de efeito. |

---

## `src/copaiba/` — Copaiba Voicebank Toolkit

| Arquivo | Descrição |
| --- | --- |
| `src/copaiba/mod.rs` | Módulo central do Copaiba: estruturas de dados de edição de amostras, aliases e gerenciamento de arquivos. |
| `src/copaiba/gui.rs` | Interface visual do Copaiba: renderização gráfica da forma de onda da amostra WAV, réguas de calibração interativa dos parâmetros do `oto.ini` (*Offset*, *Consonant*, *Cutoff*, *Preutterance*, *Overlap*) com arraste de linhas e teste de reprodução por fatia. |

---

## `src/drivers/` — Drivers de Motores (Resamplers & Wavtools)

| Arquivo | Descrição |
| --- | --- |
| `src/drivers/mod.rs` | Módulo central de drivers. Enumera motores conhecidos e define traits de interoperabilidade. |
| `src/drivers/resampler_driver.rs` | Trait `ResamplerDriver` e implementações dos motores de afinação: `NativeVenusResamplerDriver` (VENUS), `ExternalResamplerDriver` (straycat-rs e outros via CLI) e `MacResDriver`. |
| `src/drivers/wavtool_driver.rs` | Trait `WavtoolDriver` e implementações dos motores de junção: `NativeWavtoolDriver` (Andromeda) e `WavtoolYawuDriver` / `ExternalWavtoolDriver` (via CLI). |
| `src/drivers/process.rs` | Utilitários de invocação de subprocessos do sistema operacional com suporte a execução direta e Wine no Linux/macOS. |

---

## `src/dsp/` — Processamento Digital de Sinais (DSP)

| Arquivo | Descrição |
| --- | --- |
| `src/dsp/mod.rs` | Módulo raiz do pipeline DSP. Exporta algoritmos de afinação, detecção de pitch e envelopes. |
| `src/dsp/venus.rs` | **Motor VENUS:** Sintetizador vocal nativo em Rust. Análise de periodicidade com YIN, interpolação espectral, modelagem formântica e síntese por fase mínima. |
| `src/dsp/autopitch.rs` | Sistema de afinação orgânica automática (*Auto-Pitch*): gera portamentos, overshoot de ataque e vibratos dinâmicos ajustados ao andamento. |
| `src/dsp/pitch.rs` | Estrutura e manipulação contínua de curvas de pitch bend, interpolação linear, Bézier e curvas em S. |
| `src/dsp/pitch_bend.rs` | Cálculo e aplicação de modulação de frequência de notas individuais. |
| `src/dsp/pitch_encoder.rs` | Codificação e decodificação de curvas de afinação UTAU (`PBType`, `PBDst`, base64 e valores numéricos). |
| `src/dsp/envelope.rs` | Gerador e interpolador de envelopes UTAU de amplitude em 5 pontos (`p1`, `p2`, `p3`, `p4`, `p5`, `v1`, `v2`, `v3`, `v4`, `v5`). |
| `src/dsp/resampler.rs` | Utilitários e funções auxiliares de reamostragem, transposição e conversão de taxa de amostragem. |
| `src/dsp/pyin.rs` | Implementação do algoritmo Probabilistic YIN (pYIN) para rastreamento fundamental e contorno de pitch em gravações de áudio. |
| `src/dsp/lpc.rs` | Algoritmos de Predição Linear (LPC - *Linear Predictive Coding*) para extração e manipulação do envelope formântico vocal. |
| `src/dsp/sola.rs` | Rotinas auxiliares de sincronismo e sobreposição de amostras (*Synchronized Overlap-Add*). |
| `src/dsp/windowed_sinc.rs` | Interpolação de amostras com filtro Sinc Janelado para reamostragem de alta fidelidade sem perda de altas frequências. |

---

## `src/formats/` — Conversores e Parsers de Formatos

| Arquivo | Descrição |
| --- | --- |
| `src/formats/mod.rs` | Módulo raiz de formatos de arquivo. Despacha rotinas universais de importação e exportação. |
| `src/formats/aps.rs` | Parser e serializador do **Arquivo de Projeto Saturno (`.aps`)**, formato nativo do Kamafeu em JSON. |
| `src/formats/ustx.rs` | Importador e exportador do formato **OpenUtau (`.ustx`)** em YAML. |
| `src/formats/ust.rs` | Importador e exportador do formato clássico **UTAU Sequence Text (`.ust`)**. |
| `src/formats/midi.rs` | Leitor e gravador de **Standard MIDI Files (`.mid`, `.midi`)** via crate `midly`. |
| `src/formats/ufdata.rs` | Importador e exportador do esquema universal **UtaFormatix Data (`.ufdata`)**. |
| `src/formats/svp.rs` | Importador de projetos do **Synthesizer V (`.svp`)**. |
| `src/formats/vsqx.rs` | Importador de arquivos XML de sequência do **VOCALOID 3/4 (`.vsqx`)**. |

---

## `src/gui/` — Interface Gráfica de Usuário (`egui`)

### Componentes Principais em `src/gui/`

| Arquivo | Descrição |
| --- | --- |
| `src/gui/mod.rs` | Estrutura principal da aplicação gráfica `KamafeuStudioApp`, ciclo de atualização (`update`) e integração com `eframe`. |
| `src/gui/app_frame.rs` | Desenho da moldura da janela, barra de título personalizada e botões de controle de janela. |
| `src/gui/editor_canvas.rs` | Área central de renderização do piano roll, timeline métrica e visualizador de forma de onda. |
| `src/gui/editor_panels.rs` | Orquestração dos painéis laterais retráteis (arranjos, inspetor, régua). |
| `src/gui/toolbar.rs` | Barra de ferramentas superior: seleção de ferramentas (Ponteiro, Lápis, Pitch, Corte, Borracha), quantização de grid e snap. |
| `src/gui/transport.rs` | Controles de reprodução: Play, Stop, Loop, Gravação, BPM, Métrica e posição de tempo em ms / compassos. |
| `src/gui/arrangement.rs` | Painel de arranjo multifaixa: listagem de faixas vocais, controles de Volume, Pan, Mute, Solo e seleção de cantor. |
| `src/gui/inspector.rs` | Painel inspetor lateral direito: propriedades da nota selecionada, parâmetros OTO e expressões dinâmicas. |
| `src/gui/left_panel.rs` | Painel lateral esquerdo: biblioteca de cantores, lista de voicebanks e navegador de arquivos. |
| `src/gui/right_panel.rs` | Painel lateral complementar para inspeção e propriedades avançadas. |
| `src/gui/phoneme_ruler.rs` | Régua inferior de fonemas: limites visuais do `oto.ini` (*preutterance*, *overlap*, consoante fixa e corte). |
| `src/gui/phoneme_palette.rs` | Paleta de atalhos e seleção rápida de fonemas disponíveis no voicebank ativo. |
| `src/gui/history.rs` | Gerenciamento da pilha de histórico de ações (Desfazer / Refazer). |
| `src/gui/playback.rs` | Lógica de interação entre a interface gráfica e o reprodutor `AudioPlayer`. |
| `src/gui/audio_export.rs` | Rotinas de diálogo e barra de progresso para exportação de mixagens finais para WAV ou FLAC. |
| `src/gui/engine_setup.rs` | Rotinas de inicialização, detecção de motores instalados e configuração dos drivers ativos. |
| `src/gui/keyboard_shortcuts.rs` | Processamento global de eventos de teclado e execução de atalhos. |
| `src/gui/theme.rs` | Definições de estilo visual, paletas de cores, arredondamento de cantos e espaçamentos do `egui`. |
| `src/gui/fonts.rs` | Carregador e configurador de tipografia (família de fontes Outfit). |
| `src/gui/i18n.rs` | Sistema de internacionalização e dicionário multilíngue (Português, Inglês e Japonês). |
| `src/gui/image_cache.rs` | Cache de texturas e imagens em memória para renderização de avatares de cantores. |
| `src/gui/notifications.rs` | Sistema de notificações toast sobrepostas na tela para avisos e confirmações. |
| `src/gui/render_log.rs` | Janela de inspeção do log de renderização e mensagens dos subprocessos. |
| `src/gui/startup.rs` | Janela de boas-vindas, lista de projetos recentes e assistente de inicialização. |
| `src/gui/types.rs` | Tipos auxiliares, enums e estruturas de estado exclusivas da camada gráfica. |
| `src/gui/window_icon.rs` | Decodificador e configurador do ícone nativo da janela do sistema operacional. |
| `src/gui/format_actions.rs` | Ações de interface para conversão e exportação rápida de formatos. |
| `src/gui/note_actions.rs` | Ações em lote sobre notas selecionadas (quantização, transposição, normalização). |
| `src/gui/voicebank_actions.rs` | Ações de interface para carregamento, recarregamento e empacotamento de voicebanks. |
| `src/gui/background_tasks.rs` | Despacho de tarefas assíncronas em segundo plano com notificações de progresso. |
| `src/gui/marquee.rs` | Lógica da caixa de seleção retangular (*marquee selection*) no piano roll. |
| `src/gui/activity.rs` | Monitor de atividade do sistema e medidores de processamento. |

### `src/gui/piano_roll/` — Piano Roll e Gaveta de Parâmetros

| Arquivo / Subpasta | Descrição |
| --- | --- |
| `src/gui/piano_roll/mod.rs` | Módulo central do piano roll: desenho das teclas, grade métrica e notas. |
| `src/gui/piano_roll/grid.rs` | Renderização da grade tonal (linhas de semitons) e métrica temporal (compassos/tempos). |
| `src/gui/piano_roll/state.rs` | Estado transitório de arraste de notas, seleção, zoom e scroll. |
| `src/gui/piano_roll/context_menu.rs` | Menu de contexto acionado pelo clique direito sobre notas e na área de trabalho. |
| `src/gui/piano_roll/minimap.rs` | Mini-mapa de navegação rápida da visão geral do projeto. |
| `src/gui/piano_roll/note_properties.rs` | Caixa rápida de propriedades sobre a nota focada. |
| `src/gui/piano_roll/vibrato_button.rs` | Controle rápido de adição e ajuste de parâmetros de vibrato na nota. |
| `src/gui/piano_roll/phoneme_cache.rs` | Cache dos rótulos visuais de fonemas para evitar recálculos por frame. |
| `src/gui/piano_roll/interaction_tests.rs` | Testes automatizados das ações de interação com o piano roll. |
| `src/gui/piano_roll/parameter_drawer.rs` | Gaveta retrátil inferior de automação e desenho de parâmetros. |
| `src/gui/piano_roll/parameter_drawer/expressions.rs` | Edição gráfica das curvas de dinâmica, modulação, sopro, gênero e velocity. |
| `src/gui/piano_roll/parameter_drawer/expressions/tests.rs` | Testes unitários do editor de expressões. |
| `src/gui/piano_roll/parameter_drawer/envelope.rs` | Edição interativa do envelope de amplitude UTAU de 5 pontos. |

### `src/gui/menu_bar/` — Menus Superiores

| Arquivo | Descrição |
| --- | --- |
| `src/gui/menu_bar.rs` | Barra de menu superior unificada. |
| `src/gui/menu_bar/file.rs` | Menu **Arquivo**: Novo, Abrir, Salvar, Importar, Exportar Áudio e Sair. |
| `src/gui/menu_bar/edit.rs` | Menu **Editar**: Desfazer, Refazer, Cortar, Copiar, Colar, Selecionar Tudo e Transposição. |
| `src/gui/menu_bar/view.rs` | Menu **Exibir**: Alternar visibilidade de painéis, zoom e modo tela cheia. |
| `src/gui/menu_bar/tracks.rs` | Menu **Faixas**: Adicionar, duplicar, excluir e gerenciar faixas vocais. |
| `src/gui/menu_bar/singers.rs` | Menu **Cantores**: Gerenciar voicebanks, instalar pacotes `.kfv` e abrir o Copaiba. |
| `src/gui/menu_bar/vocal_modes.rs` | Menu **Modos Vocais**: Seleção do fonemizador (Japonês, Português, Inglês, Manual). |
| `src/gui/menu_bar/tools.rs` | Menu **Ferramentas**: Auto-Pitch, humanização, afinador orgânico e utilitários. |
| `src/gui/menu_bar/playback.rs` | Menu **Reprodução**: Play, Stop, Loop, Metrônomo e opções de buffer. |
| `src/gui/menu_bar/language.rs` | Menu **Idioma**: Troca instantânea entre Português, Inglês e Japonês. |
| `src/gui/menu_bar/help.rs` | Menu **Ajuda**: Atalhos de teclado, guia de uso, verificação de atualizações e sobre. |

### `src/gui/unified_panel/` — Painel Unificado Lateral

| Arquivo | Descrição |
| --- | --- |
| `src/gui/unified_panel.rs` | Container do painel unificado retrátil. |
| `src/gui/unified_panel/singer.rs` | Aba de configuração do cantor ativo, foto, prefix.map e informações. |
| `src/gui/unified_panel/note.rs` | Aba de ajuste fino da nota musical selecionada. |
| `src/gui/unified_panel/phonemes.rs` | Aba de inspeção dos fonemas da nota e modificação manual de aliases. |
| `src/gui/unified_panel/engine.rs` | Aba de seleção de motores de áudio (Resamplers: VENUS, straycat-rs / Wavtools: Andromeda, Yawu). |

### `src/gui/dialogs/` — Janelas Modais e Diálogos

| Arquivo | Descrição |
| --- | --- |
| `src/gui/dialogs/autopitch.rs` | Diálogo interativo de calibração do gerador de afinação orgânica (*Auto-Pitch*). |
| `src/gui/dialogs/export_dialog.rs` | Diálogo de exportação com opções de formato (WAV/FLAC), taxa de amostragem e multifaixa. |
| `src/gui/dialogs/export_options_dialog.rs` | Configurações avançadas de renderização de exportação. |
| `src/gui/dialogs/fx_rack_dialog.rs` | Janela flutuante do rack de efeitos de áudio (EQ 31 bandas, Compressor, Chorus, Reverb). |
| `src/gui/dialogs/shortcuts_guide.rs` | Janela com o guia completo de teclas de atalho do sistema. |
| `src/gui/dialogs/singers_gallery.rs` | Galeria visual de cantores e voicebanks instalados. |
| `src/gui/dialogs/theme_customizer.rs` | Painel de personalização em tempo real de temas e cores da interface. |
| `src/gui/dialogs/theme_editor_dialog.rs` | Editor de paletas de cores e temas visuais do aplicativo. |
| `src/gui/dialogs/humanize_dialog.rs` | Diálogo de humanização de notas (pequenas variações orgânicas de tempo e pitch). |
| `src/gui/dialogs/lyrics_dialog.rs` | Inserção rápida de letras de música em lote. |
| `src/gui/dialogs/batch_lyrics.rs` | Processamento e divisão em lote de textos e letras de músicas. |
| `src/gui/dialogs/copaiba.rs` | Janela modal incorporada do utilitário Copaiba. |
| `src/gui/dialogs/voicebank.rs` | Diálogo de propriedades e detalhes do banco de voz. |
| `src/gui/dialogs/folder_picker.rs` | Seletor nativo/incorporado de diretórios de voicebanks e projetos. |
| `src/gui/dialogs/project.rs` | Configurações de propriedades do projeto ativo. |
| `src/gui/dialogs/templates.rs` | Seleção de modelos pré-configurados de projeto. |

### `src/gui/preferences_dialog/` — Painel de Preferências

| Arquivo | Descrição |
| --- | --- |
| `src/gui/preferences_dialog.rs` | Janela principal de preferências do sistema. |
| `src/gui/preferences_dialog/audio_settings.rs` | Configurações de drivers de áudio, dispositivo de saída e tamanho de buffer. |
| `src/gui/preferences_dialog/dsp_settings.rs` | Configurações do algoritmo VENUS, alinhamento de fase e equal-power crossfade. |
| `src/gui/preferences_dialog/voicebank_tuning.rs` | Ajustes globais de tolerância de timing e resolução de aliases do OTO. |
| `src/gui/preferences_dialog/memory_cache.rs` | Gerenciamento de limite de memória RAM para cache de amostras WAV e fonemas. |
| `src/gui/preferences_dialog/ui_workflow.rs` | Preferências de layout, comportamento do cursor e atalhos. |
| `src/gui/preferences_dialog/export_defaults.rs` | Padrões de taxa de amostragem e formato de exportação. |
| `src/gui/preferences_dialog/diagnostics.rs` | Ferramentas de diagnóstico do sistema e status dos motores. |
| `src/gui/preferences_dialog/presets_and_help.rs` | Perfis pré-definidos de configuração e ajuda. |
| `src/gui/preferences_dialog/experimental.rs` | Habilitação de funcionalidades experimentais em teste. |

---

## `src/oto/` — Sistema de Calibração OTO e Voicebanks

| Arquivo | Descrição |
| --- | --- |
| `src/oto/mod.rs` | Módulo raiz do subsistema OTO. |
| `src/oto/entry.rs` | Estrutura de dados `OtoEntry`: armazena *Offset*, *Consonant*, *Cutoff*, *Preutterance*, *Overlap* e alias do fonema. |
| `src/oto/parser.rs` | Parser de alta performance de arquivos `oto.ini` com suporte automático a codificações UTF-8 e Shift-JIS. |
| `src/oto/prefix_map.rs` | Parser e resolutor de tabelas `prefix.map` para seleção da amostra ideal baseada na nota MIDI musical. |
| `src/oto/voicebank.rs` | Modelo completo de um cantor virtual: árvore de subpastas, metadados (`character.txt`), tabela geral de fonemas e ícone. |
| `src/oto/singers.rs` | Gerenciador e indexador de múltiplos cantores instalados no sistema. |

---

## `src/phonemizer/` — Motores de Conversão Fonética

| Arquivo | Descrição |
| --- | --- |
| `src/phonemizer/mod.rs` | Módulo raiz e trait unificada `Phonemizer` para resolução de letras em cadeias fonéticas. |
| `src/phonemizer/japanese.rs` | Fonemizador Japonês: suporte a bancos CV (Hiragana), VCV e CVVC. |
| `src/phonemizer/romaji.rs` | Conversor ortográfico Romaji para Kana (Hiragana / Katakana). |
| `src/phonemizer/portuguese.rs` | Fonemizador Português Brasileiro (G2P - Grafema para Fonema). |
| `src/phonemizer/brapa.rs` | Implementação do padrão fonético brasileiro **BRAPA VCCV**, CVC e CVVC. |
| `src/phonemizer/english.rs` | Fonemizador Inglês com suporte a encontros consonantais complexos. |
| `src/phonemizer/vccv.rs` | Lógica geral para encadeamento e segmentação de transições VCCV (Vogal-Consoante-Consoante-Vogal). |

---

## `src/project/` — Modelagem de Dados e Projeto

| Arquivo | Descrição |
| --- | --- |
| `src/project/mod.rs` | Módulo raiz do modelo de projeto. |
| `src/project/model.rs` | Estruturas de dados principais da DAW: `Project`, `Track`, `Part`, `Note`, `Phoneme`, `PitchCurve`, `Envelope` e parâmetros de automação musical. |

---

## `src/renderer/` — Pipeline de Renderização e Síntese de Áudio

| Arquivo / Subpasta | Descrição |
| --- | --- |
| `src/renderer/mod.rs` | Módulo raiz do renderizador de síntese vocal. |
| `src/renderer/project.rs` | Orquestrador de renderização de projetos multifaixa. |
| `src/renderer/chunked.rs` | Renderização assíncrona por blocos progressivos (*chunks*) em paralelo via pool de threads **Rayon**. |
| `src/renderer/track.rs` | Renderização de faixa vocal individual: coleta de notas, resolução de fonemas e chamada aos drivers. |
| `src/renderer/track/mixing.rs` | Mixagem e junção de fatias sintetizadas com interpolação e crossfade de potência constante (*equal-power*). |
| `src/renderer/track/phrase_pitch.rs` | Cálculo contínuo da curva de pitch ao longo de frases completas. |
| `src/renderer/track/wav_io.rs` | Rotinas de gravação de arquivos temporários e leitura de buffers de áudio gerados pelos motores. |
| `src/renderer/track/tests.rs` | Testes unitários do pipeline de renderização e mixagem de faixas. |
| `src/renderer/timing.rs` | Motor de temporização: calcula posições acústicas exatas aplicando *preutterance* e *overlap* nas notas musicais. |
| `src/renderer/phase.rs` | Algoritmos de alinhamento de fase entre fonemas adjacentes para eliminação de cancelamentos harmônicos e estalos. |
| `src/renderer/resampler_cache.rs` | Sistema de cache em disco e memória RAM para trechos de áudio já sintetizados (evita re-renderizações desnecessárias). |
| `src/renderer/exporter.rs` | Exportador de áudio multithread para master final e stems de faixas individuais em WAV/FLAC. |
| `src/renderer/options.rs` | Estrutura de opções e parâmetros de renderização (taxa de amostragem, número de threads, modos de qualidade). |
