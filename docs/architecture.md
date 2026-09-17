# Mapa de manutenção do Kamafeu Studio

O objetivo desta organização é permitir investigar um comportamento sem carregar
o aplicativo inteiro. Os caminhos públicos existentes foram preservados: por
exemplo, `gui::KamafeuStudioApp`, `gui::piano_roll::draw_piano_roll`,
`audio::fx::FxRackProcessor` e `renderer::TrackRenderer`.

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
