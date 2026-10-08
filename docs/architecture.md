# Mapa de manutenção do Kamafeu Studio

O contrato de extensões e seu fluxo de declaração estão documentados em
[docs/extensoes.md](extensoes.md).
O guia para autores de plugins está em [docs/plugin-sdk.md](plugin-sdk.md).

O contrato APS é versionado por `UProject.schema_version` (atualmente v4).
Arquivos sem esse campo são tratados como legados e normalizados para a versão
atual; APS v1 sem `extensions` recebe uma migração determinística que inicializa
essa lista vazia e registra um aviso. O parser oferece
`parse_str_with_report` para interfaces que precisam mostrar esses avisos sem
alterar a origem.
APS v2 é atualizado para v3 preservando projetos antigos; APS v3 recebe a
migração v4 para o estado de renderização progressiva. Os controles por fonema
ausentes permanecem em herança e cada migração registra esse fato.

O caminho de voz deve permanecer único: fonemização, resolução no `oto.ini`,
timing, flags, resampling e wavtool formam o mesmo plano de render. Um
voicebank com `oto.ini` continua sendo UTAU mesmo se possuir uma pasta auxiliar
`dsvocoder`; DiffSinger exige sua configuração e modelos próprios.
Essa regra é coberta por um teste de renderização real com o alias `k a`, não
apenas por teste de descoberta do arquivo.

`project::ProjectDiff::between` compara revisões pelo modelo musical, incluindo
metadados relevantes, arranjo, configurações de render, notas, overrides de
fonemas e partes de áudio. Ele não
compara o JSON bruto: migrações, formatação e ordem de campos não produzem
alterações falsas. A operação é somente leitura e prepara a comparação visual
e a restauração seletiva por parte. O resultado também inclui os índices
estáveis de notas e partes de áudio alteradas, para que consumidores não
precisem recalcular a localização a partir do JSON.
`project::restore_note_indices` aplica uma seleção validada de notas sobre uma
cópia do projeto atual, preservando arranjo, áudio e configuração de render;
o chamador deve registrar o snapshot anterior no undo antes de confirmar a
operação.
`project::restore_wave_part_indices` oferece a mesma transação para descritores
de partes de áudio, sem tocar no material vocal.
`restore_marker_indices` e `restore_section_indices` completam o mesmo contrato
para o arranjo; todos os helpers são puros em relação aos argumentos e deixam a
política de confirmação/undo para a camada de aplicação.

O objetivo desta organização é permitir investigar um comportamento sem carregar
o aplicativo inteiro. Os caminhos públicos existentes foram preservados: por
exemplo, `gui::KamafeuStudioApp`, `gui::piano_roll::draw_piano_roll`,
`audio::fx::FxRackProcessor` e `renderer::TrackRenderer`.

`UProject::diagnostic_report` é não destrutivo e cada `ProjectIssue` oferece
uma sugestão textual estável (`suggestion()`). A GUI mostra a sugestão e só
abre o fluxo de correção após confirmação; automação e CLI recebem o mesmo
campo no JSON, sem aplicar reparos implicitamente.
O relatório de voicebank segue o mesmo contrato: problemas de WAV e timing do
`oto.ini` expõem uma ação sugerida para restauração ou recalibração, sem editar
arquivos automaticamente.

Flags UTAU são mescladas por token na ordem projeto, faixa, perfil de render e
nota/fonema. O valor mais específico substitui apenas o mesmo token; flags que
Kamafeu não conhece continuam presentes na linha final enviada ao resampler.
O helper `set_utau_flag` permite alterar/remover um token estruturado sem
descartar os demais, mantendo a edição literal como fonte de compatibilidade.
O inspetor de nota usa esse helper para as principais flags numéricas (`g`, `B`,
`b`, `C`, `D`, `E`, `e`, `H`, `Hb`, `Mb`, `Mt`, `N`, `O`, `P`, `R`, `S`, `T`,
`V`, `W` e `Y`), enquanto tokens não reconhecidos permanecem editáveis na linha
literal.
Overrides de fonema também podem fornecer uma linha literal de flags; ela é
mesclada depois das flags da nota, mantendo tokens desconhecidos.
O mesmo override pode substituir, sem alterar a nota original, o alias final,
envelope UTAU, vibrato e curva de pitch/portamento daquele subfonema. A
precedência é: valor calculado pelo fonemizador/nota, depois override do
fonema; campos não informados continuam herdando o valor calculado. Esses
campos são persistidos no APS e chegam ao `RenderPhone` antes do timing e do
resampler.
Na GUI, a opção de escopo avançado aparece quando um fonema está selecionado;
desmarcada, a edição continua sendo por nota. Ao ativá-la, envelope, vibrato,
portamento e offset de consoante são gravados no override do fonema, deixando
visível a diferença entre o valor herdado e o valor específico.
Quando o alias selecionado corresponde a uma divisão persistida da letra, o
painel de nota expõe esse override e mostra a linha efetiva antes do resampler.
Para investigar uma resolução sem renderizar, a CLI aceita
`kamafeu voicebank-info <banco> --alias "k a" --pitch C4 --json` e retorna o
alias final, WAV e todos os valores de timing encontrados.
Durante o render, cada fonema também registra uma descrição segura dos comandos
de resampler e wavtool, incluindo caminhos, flags e parâmetros temporais; essa
linha serve para diagnóstico e não é executada como shell.
O mesmo fluxo mede RMS em janelas de 10 ms no início e no fim do buffer de cada
fonema. Essas observações vão para `timing_boundary_diagnostics` no manifesto e
podem gerar somente avisos para RMS não finito ou para duas extremidades
silenciosas; nunca fazem nivelamento automático nem mudam o áudio.
Executáveis externos também entram no cache por fingerprint de conteúdo, para
que uma substituição no mesmo caminho nunca reutilize áudio antigo.
O fingerprint do voicebank segue a mesma regra e inclui aliases, timing do
`oto.ini` e bytes dos WAVs referenciados.
Os WAVs existentes também aparecem como artefatos nomeados no sidecar; entradas
ausentes continuam sendo reportadas pelo fingerprint e pelo diagnóstico sem
impedir a proveniência do export.

Manifestos de render podem registrar artefatos de entrada com
`RenderProvenance::with_artifact_file`. O fingerprint é calculado sobre o
conteúdo do arquivo, não sobre timestamp ou caminho; isso permite invalidar
cache quando `oto.ini`, modelos, executáveis ou outros recursos realmente
mudam.
O render headless da CLI grava automaticamente esse manifesto ao lado do áudio
exportado (`<arquivo>.<extensão>.kamafeu.json`); exportações pela GUI continuam
podendo optar pelo mesmo helper sem alterar o formato do áudio.

Pré-visualizações progressivas publicam `ProgressiveChunk` com índice
zero-based e início temporal. O índice permanece estável dentro da sequência,
 inclusive em chunks de erro, permitindo repetição seletiva
identifique o trecho sem inferir a posição a partir do tamanho do buffer.
`ProgressiveChunk::end_ms()` fornece o limite final usando frames e sample rate;
o terminal filtra esses eventos pela categoria `Chunks`.

Referências de extensão persistidas em APS podem ser comparadas ao catálogo
descoberto com `extensions::diagnose_project_extensions`. A comparação é
somente metadata e informa separadamente extensão ausente, versão divergente e
manifesto alterado; nenhum módulo WASM é executado durante esse diagnóstico.

## Onde procurar

| Comportamento | Arquivo ou diretório em `src/` |
| --- | --- |
| Estado do aplicativo | `gui/mod.rs` |
| Inicialização e seleção de motores | `gui/startup.rs`, `gui/engine_setup.rs` |
| Ordem das operações por frame | `gui/app_frame.rs` |
| Mensagens de renderização, exportação e transporte | `gui/background_tasks.rs` |
| Atalhos e Undo/Redo | `gui/keyboard_shortcuts.rs`, `gui/history.rs` |
| Edição de notas | `gui/note_actions.rs` |
| Abrir, salvar, snapshots e modelos de projeto | `gui/project_files.rs` |
| Importar e exportar formatos de projeto | `gui/format_actions.rs` |
| Reprodução e prévia de waveform | `gui/playback.rs` |
| Exportação de áudio | `gui/audio_export.rs` |
| Recarregar voicebank e integração com Copaiba | `gui/voicebank_actions.rs` |
| Painéis e transações de edição do canvas | `gui/editor_panels.rs`, `gui/editor_canvas.rs` |
| Menus do aplicativo | `gui/menu_bar/` |
| Janelas e diálogos | `gui/dialogs/` |
| Preferências por categoria | `gui/preferences_dialog/` |
| Abas Cantor, Nota, Fonemas e Motor | `gui/unified_panel/` |
| Piano roll e interação com notas | `gui/piano_roll/mod.rs` |
| Menu, propriedades e minimapa do piano roll | `gui/piano_roll/context_menu.rs`, `note_properties.rs`, `minimap.rs` |
| Envelopes e expressões | `gui/piano_roll/parameter_drawer/` |
| Cache de fonemas do piano roll | `gui/piano_roll/phoneme_cache.rs` |
| Configuração serializável dos efeitos | `audio/fx/settings.rs` |
| Ordem e estado dos efeitos de áudio | `audio/fx/processor.rs` |
| Equalizador e coeficientes biquad | `audio/fx/equalizer.rs`, `audio/fx/biquad.rs` |
| Renderização de faixa | `renderer/track.rs` |
| Pitch compartilhado da frase | `renderer/track/phrase_pitch.rs` |
| Mixagem e correção de fase | `renderer/track/mixing.rs`, `renderer/phase.rs` |

## Diagnóstico de timing vocal

`renderer::timing::diagnostics` é a representação serializável da resolução de
timing por fonema. Ela preserva, no mesmo registro, os valores de origem do
`oto.ini`, os deltas/overrides aplicados e a geometria final (`preutter`,
`overlap`, `tail_intrude` e `tail_overlap`).

`UProject::diagnostic_report` também inspeciona overrides persistidos por
fonema. A validação é não destrutiva: índices incompatíveis, aliases vazios e
valores não finitos são emitidos como avisos com localização da nota, enquanto
a normalização só acontece no fluxo explícito de salvar/migrar. A CLI
`validate-project`, a automação JSON e o diálogo de diagnóstico da GUI exibem o
mesmo `kind`, localização e detalhe, portanto novos tipos não podem ser
silenciosamente descartados por uma camada de apresentação.

O fluxo invariável é:

```text
oto.ini + expressão manual → PhonemeTimingInput → resolve_phoneme_timings
                           → PhonemeTimingDiagnostic (JSON/log/UI)
```

Prévia, render final e ferramentas de diagnóstico devem consumir essa mesma
resolução; nenhum painel deve recalcular preutterance ou overlap localmente.
Durante um render, cada registro é emitido no fluxo de progresso com o prefixo
`[Timing]` e corpo JSON. Consumidores podem filtrar esse prefixo para construir
uma auditoria por fonema sem interpretar mensagens humanas de progresso ou
erros.

Prévia isolada usa `renderer::PreviewTarget` e
`ProjectRenderer::preview_bounds`, que calculam o intervalo absoluto para
trechos, notas, subfonemas e transições. A mesma geometria deve ser passada a
`render_project_range_with_drivers_cancellable`, evitando que a GUI e o render
final escolham limites diferentes. Para obter áudio já recortado ao alvo, use
`ProjectRenderer::render_preview_target_with_drivers`; ele preserva contexto
vocal anterior e devolve exatamente a duração selecionada.

A auditoria de conversão compara também a quantidade e a assinatura dos
overrides por fonema. Formatos que não possuem representação equivalente, como
UST clássico, geram aviso explícito; `convert --fail-on-loss` falha nessa
situação para CI, em vez de declarar um round-trip preservado apenas porque o
número de notas permaneceu igual.
| Leitura e escrita WAV | `renderer/track/wav_io.rs` |
| Regressões de áudio | `renderer/track/tests.rs`, `audio/fx/tests.rs`, `tests/engine_contract.rs` |

## Contratos a preservar

- A ordem das etapas de `app_frame.rs` importa: receber tarefas, processar
  atalhos, desenhar painéis/canvas, apresentar diálogos e atualizar a atividade.
- O snapshot de edição precisa existir antes da mutação da nota. Os callbacks
  de início e confirmação da edição continuam nos pontos originais.
- Mover um diálogo não deve alterar IDs egui, ordem de execução ou atalhos.
- A configuração dos efeitos conserva os campos e valores serializados.
- O rack continua processando estéreo intercalado na ordem EQ, compressor,
  delay e reverb. A mudança da taxa de amostragem exige criar um processador
  para a nova taxa, pois filtros e buffers são dimensionados no construtor.
- Métodos auxiliares novos devem ter a menor visibilidade necessária; manter
  a API pública existente não exige tornar públicos os módulos internos.

## Otimização do equalizador

O equalizador guarda os coeficientes por ganho e os índices das bandas ativas
em arrays fixos. Com ganhos estáveis, não recalcula trigonometria por bloco.
O loop por frame estéreo percorre somente as bandas ativas, na ordem original,
sem criar vetores no caminho de processamento.

O teste diferencial compara os resultados exatamente com o algoritmo anterior,
incluindo múltiplos blocos, alterações de ganho, desativação/reativação, limiar
de ativação, buffer de comprimento ímpar e taxas de 8, 44,1, 48 e 96 kHz.

## Validação e limites

Execute a partir da raiz:

```sh
rtk cargo fmt --all -- --check
rtk cargo clippy --all-targets --all-features -- -D warnings
rtk cargo test --all-targets --all-features
rtk git diff --check
```

A separação reduz o escopo de leitura, mas não comprova redução do tempo do
rust-analyzer ou de compilação. FPS e desempenho de renderização precisam de
medições com projetos representativos. Os testes não substituem a conferência
visual de arrastes, Undo/Redo, diálogos e reprodução na aplicação aberta.

Ainda há funções extensas no desenho/interação das notas do piano roll e em
algumas abas de propriedades. A organização atual não representa uma auditoria
de todos os algoritmos de DSP nem uma garantia de ausência de bugs.
