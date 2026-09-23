# Guia do código-fonte do Kamafeu Studio

Este documento descreve, em português, a organização do código publicado no
repositório. Os comentários dentro dos módulos explicam os invariantes locais;
este guia explica como as partes se conectam.

## Pontos de entrada

| Arquivo | Responsabilidade |
| --- | --- |
| `src/main.rs` | Inicializa o aplicativo desktop e encaminha a execução para a interface. |
| `src/lib.rs` | Declara os módulos públicos da biblioteca e os metadados da versão. |
| `src/bin/copaiba.rs` | Executável independente do toolkit de voicebanks Copaiba. |
| `src/bin/venus-editor.rs` | Ferramenta de terminal para analisar e editar arquivos `.venus`. |
| `src/bin/venus-corpus.rs` | Mede um corpus real de voicebank e gera relatório de F0, RMS, pico e descontinuidades. |
| `index.html` | Entrada do build WebAssembly usado pelo Trunk. |
| `Trunk.toml` | Configura o empacotamento do aplicativo WebAssembly. |

## Dados, configuração e formatos

| Caminho | Responsabilidade |
| --- | --- |
| `src/config.rs` | Preferências persistentes, temas, caminhos de motores, idioma e cache. |
| `src/project/model.rs` | Modelo de projeto: faixas, partes, notas, áudio, expressões e operações de tempo. |
| `src/project/mod.rs` | Integração do modelo com histórico e serviços de projeto. |
| `src/formats/aps.rs` | Formato nativo `.aps`. |
| `src/formats/ust.rs` | Importação e exportação do UST clássico. |
| `src/formats/ustx.rs` | Compatibilidade YAML com projetos OpenUtau. |
| `src/formats/ufdata.rs` | Intercâmbio UtaFormatix. |
| `src/formats/midi.rs` | Leitura e escrita de Standard MIDI, incluindo notas e andamento. |
| `src/formats/svp.rs` | Conversão de projetos Synthesizer V. |
| `src/formats/vsqx.rs` | Conversão de sequências Vocaloid VSQX. |
| `src/formats/mod.rs` | Registro público dos formatos disponíveis. |

## Áudio e síntese

| Caminho | Responsabilidade |
| --- | --- |
| `src/audio/loader.rs` | Carregamento e inspeção de arquivos de áudio. |
| `src/audio/player.rs` | Reprodução, buffers, volume e saída de áudio. |
| `src/audio/metronome.rs` | Geração do metrônomo sincronizado ao BPM. |
| `src/audio/fx/` | Processadores de equalização, compressor, chorus, delay e reverb. |
| `src/drivers/resampler_driver.rs` | Descoberta, configuração e execução de resamplers UTAU externos. |
| `src/drivers/wavtool_driver.rs` | Integração com wavtools e o motor nativo de junção. |
| `src/drivers/process.rs` | Execução controlada de processos externos. |
| `src/dsp/venus.rs` | Análise de F0, vozeamento e resampling nativo VENUS. |
| `src/dsp/venus_analysis.rs` | Estruturas e descritores persistidos no arquivo `.venus`. |
| `src/dsp/venus_reference_tests.rs` | Casos de referência para estabilidade do VENUS. |
| `src/dsp/pitch.rs` | Conversão entre notas MIDI, frequência e nomes de nota. |
| `src/dsp/pitch_bend.rs` | Solução e interpolação de curvas de afinação. |
| `src/dsp/pitch_encoder.rs` | Codificação de pitch bend para formatos e resamplers UTAU. |
| `src/dsp/autopitch.rs` | Geração de portamentos, vibratos e curvas orgânicas. |
| `src/dsp/envelope.rs` | Envelopes de volume e envelopes UTAU de cinco pontos. |
| `src/dsp/resampler.rs` | Rotinas gerais de reamostragem e preservação de consoante. |
| `src/dsp/sola.rs` | Alongamento temporal por SOLA. |
| `src/dsp/pyin.rs` | Estimativa de frequência fundamental. |
| `src/dsp/lpc.rs` | Análise e processamento de envelope espectral. |
| `src/dsp/windowed_sinc.rs` | Interpolação de alta qualidade por sinc janelado. |

## Fonética e voicebanks

| Caminho | Responsabilidade |
| --- | --- |
| `src/oto/voicebank.rs` | Representação e carregamento de um voicebank. |
| `src/oto/parser.rs` | Leitura de `oto.ini` em codificações compatíveis. |
| `src/oto/entry.rs` | Estrutura de uma entrada de amostra e seus parâmetros temporais. |
| `src/oto/prefix_map.rs` | Seleção de prefixos/sufixos por tom e cor vocal. |
| `src/oto/singers.rs` | Catálogo e descoberta de cantores instalados. |
| `src/oto/diagnostics.rs` | Diagnóstico de aliases, arquivos ausentes e inconsistências. |
| `src/phonemizer/mod.rs` | Contrato comum, modos fonéticos, overrides e montagem de `RenderPhone`. |
| `src/phonemizer/japanese.rs` | Fonemização japonesa CV, VCV e CVVC. |
| `src/phonemizer/brapa.rs` | Fonemização BRAPA VCCV/CVC. |
| `src/phonemizer/portuguese.rs` | Regras portuguesas CVVC, VCV e G2P. |
| `src/phonemizer/english.rs` | Tokens ARPABET e fonemização inglesa. |
| `src/phonemizer/vccv.rs` | Regras de contexto VCCV. |
| `src/phonemizer/romaji.rs` | Conversão de romaji para kana. |

## Interface gráfica

| Caminho | Responsabilidade |
| --- | --- |
| `src/gui/mod.rs` | Estado principal da interface e composição dos painéis. |
| `src/gui/app_frame.rs` | Ciclo de frame, notificações e desenho geral. |
| `src/gui/editor_panels.rs` | Montagem do arranjo, piano roll, inspetor e painéis superiores. |
| `src/gui/editor_canvas.rs` | Área principal do editor e roteamento da edição. |
| `src/gui/arrangement.rs` | Faixas, partes, áudio, waveform de arranjo e playhead compartilhada. |
| `src/gui/piano_roll/mod.rs` | Renderização e interação central do piano roll. |
| `src/gui/piano_roll/navigation.rs` | Scroll, zoom, acompanhamento horizontal/vertical e gestos. |
| `src/gui/piano_roll/minimap.rs` | RADAR, janela de viewport e arraste de navegação. |
| `src/gui/piano_roll/grid.rs` | Grade temporal, snap e linhas musicais. |
| `src/gui/piano_roll/processed_waveform.rs` | Waveform vocal exibida no rodapé do piano roll. |
| `src/gui/piano_roll/phoneme_cache.rs` | Cache visual de fonemas e aliases. |
| `src/gui/piano_roll/parameter_drawer.rs` | Painel de expressões e automações. |
| `src/gui/piano_roll/parameter_drawer/expressions.rs` | Lanes de dinâmica, timbre, volume, vibrato e demais parâmetros. |
| `src/gui/piano_roll/context_menu.rs` | Ações contextuais para notas, seleção e exportação. |
| `src/gui/piano_roll/note_properties.rs` | Propriedades detalhadas da nota. |
| `src/gui/toolbar.rs` | Transporte, ferramentas de edição, snap, zoom e controles rápidos. |
| `src/gui/menu_bar.rs` | Menu global, menu compacto e identidade do projeto. |
| `src/gui/menu_bar/` | Implementação separada dos menus de arquivo, edição, faixas, modos vocais, ferramentas, exibição, playback, idioma e ajuda. |
| `src/gui/phoneme_ruler.rs` | Régua de fonemas, preutterance, overlap e envelopes. |
| `src/gui/unified_panel/` | Painéis de cantor, nota, fonemas e motor. |
| `src/gui/preferences_dialog/` | Preferências de áudio, DSP, exportação, interface, voicebank e cache. |
| `src/gui/dialogs/` | Diálogos de projeto, letras, autopitch, exportação, voicebank e temas. |
| `src/gui/keyboard_shortcuts.rs` | Atalhos globais e comandos de edição. |
| `src/gui/theme.rs` | Paletas, métricas e componentes visuais do tema. |

## Renderização e playback

| Caminho | Responsabilidade |
| --- | --- |
| `src/renderer/timing.rs` | Linha do tempo fonética, preutterance, overlap e duração acústica. |
| `src/renderer/project.rs` | Renderização progressiva e paralela do projeto. |
| `src/renderer/track.rs` | Renderização de uma faixa vocal e seus slots de fonemas. |
| `src/renderer/track/slots.rs` | Planejamento dos slots de síntese. |
| `src/renderer/track/phone_result.rs` | Resultado intermediário de um fonema renderizado. |
| `src/renderer/track/mixing.rs` | Crossfade, alinhamento de fase e mixagem da faixa. |
| `src/renderer/track/wav_io.rs` | Leitura, validação e escrita de buffers WAV. |
| `src/renderer/track/phrase_pitch.rs` | Curva de pitch de uma frase. |
| `src/renderer/resampler_cache.rs` | Cache cancelável de síntese e invalidação por parâmetros. |
| `src/renderer/chunked.rs` | Processamento por blocos. |
| `src/renderer/exporter.rs` | Exportação final de áudio. |
| `src/renderer/diagnostics.rs` | Métricas e diagnósticos de renderização. |
| `src/gui/background_tasks.rs` | Atualização do relógio de playback, render progressivo e VU meter. |
| `src/gui/playback.rs` | Início, pausa, parada, loop e preparação do áudio. |

## Testes e documentação de operação

Os testes unitários ficam próximos dos módulos que validam. Os testes de
interação do piano roll estão em
`src/gui/piano_roll/interaction_tests.rs`; os testes de parâmetros ficam em
`src/gui/piano_roll/parameter_drawer/expressions/tests.rs`; e os testes do
renderizador ficam em `src/renderer/track/tests.rs`.

Os workflows em `.github/workflows/` executam formatação, testes, Clippy,
auditoria de dependências, builds desktop e o build WebAssembly para GitHub
Pages. O diretório `dist/` não é versionado porque é produzido pelo workflow.
