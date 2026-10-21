# Arquitetura do Kamafeu Studio

O código é organizado por responsabilidade. Esta página registra a estrutura
atual e serve como guia para extrações internas que devem preservar os
contratos públicos e os formatos de projeto.

## Aplicação e automação

- `src/main.rs` é o ponto de entrada do binário nativo e delega a execução para
  `src/cli.rs`.
- `src/cli/args.rs` declara os argumentos da linha de comando, os comandos, os
  formatos de saída e o protocolo de automação JSON. Seus nomes e defaults são
  parte dos contratos com scripts.
- `src/cli/startup.rs` contém o tratamento de falhas, a seleção do renderer e a
  configuração/inicialização da janela; `cli.rs` mantém o ponto de entrada e o
  despacho dos comandos.
- `src/cli/sample.rs` gera o banco vocal de exemplo solicitado pela CLI.
- `src/cli/common.rs` concentra leitura e gravação de projetos, serialização
  JSON usada pela CLI e pela automação e normalização dos modos de divisão de
  letras.
- `src/cli/voicebank.rs` concentra inspeção, catálogo, validação e migração de
  bancos vocais acionados pela linha de comando.
- `src/cli/project.rs` concentra inspeção, diff, proveniência e validação de
  projetos; conversão e render continuam em seus módulos próprios.
- `src/cli/extension_command.rs` implementa o comando de extensão e o protocolo
  de invocação explícita; `src/cli/extensions.rs` reúne a descoberta e as
  rotinas de catálogo compartilhadas.
- `src/cli/environment.rs` agrupa informações de plataforma e as operações de
  inspeção e limpeza do cache de render.
- `src/cli.rs` mantém a inicialização da GUI e a seleção de comandos.
- `src/cli/tests.rs` reúne os testes de contrato dos comandos, automação, MCP e
  render headless, fora do módulo de despacho.
- `src/cli/automation.rs` mantém o transporte JSON por linha e as operações de
  projeto compartilhadas pela CLI e pelo MCP.
- `src/cli/conversion.rs` contém auditoria e conversão de projetos, incluindo
  os mesmos relatórios usados pela CLI e pela automação.
- `src/cli/render.rs` contém o render headless para arquivo e o resumo JSON
  comum à CLI e à automação.
- `src/cli/mcp.rs` mantém o transporte stdio JSON-RPC e o catálogo MCP; as
  chamadas continuam delegadas às mesmas operações da automação.
- `src/cli/extensions.rs` mantém inspeção de catálogo, análise de permissões,
  verificação WASM e invocação estruturada das extensões.

## Domínios da biblioteca

- `src/phonemizer/` contém o registro e as regras por idioma. O módulo japonês
  expõe sua API em `japanese.rs`; leitura de
  `presamp.ini`, inventário e expansão de aliases ficam em
  `japanese/presamp.rs`, como detalhe interno do fonemizador. A atribuição de
  fonemas e aliases do Presamp ficam em `japanese/phonemize.rs`; Japanese CV,
  VCV e CVVC compatíveis com OpenUtau ficam em `japanese/openutau.rs`; as
  tabelas de mora, lookup e substituições ficam em `japanese/tables.rs`. As
  rotinas de G2P e os dicionários compartilhados ficam em `g2p.rs`. O
  processamento de inglês e Arpasing fica em `english/phonemize.rs`; a
  conversão G2P e as regras de tokens em português ficam em `portuguese/g2p.rs`;
  a aplicação de aliases e timing CVVC/VCV fica em `portuguese.rs`. O catálogo de
  modos e metadados fica em `mode.rs`, e o dispatcher público em
  `dispatcher.rs`. O adaptador de compatibilidade do módulo raiz está isolado
  em `legacy.rs`; reexports preservam os nomes históricos e os módulos por
  idioma continuam usando os helpers necessários. As fixtures existentes
  continuam cobrindo aliases.
- `src/gui/` e `src/copaiba/` contêm a interface e a edição de projeto. No
  piano roll, a régua e suas interações de tempo ficam em
  `src/gui/piano_roll/ruler.rs`; as linhas de escala e tonalidade ficam em
  `src/gui/piano_roll/grid.rs`; Pointer e Pencil editam notas por meio de
  `src/gui/piano_roll/note_interaction.rs`, enquanto a curva, as prévias e a
  edição de pontos de pitch ficam em `src/gui/piano_roll/pitch_interaction.rs`;
  suavização do traço, inserção de pontos e normalização de formas ficam em
  `src/gui/piano_roll/pitch_data.rs`, com reexports em `state.rs`.
  Acumulação e consulta do waveform de prévia ficam em
  `src/gui/piano_roll/state/waveform.rs`; o setter público da resolução segue
  reexportado pelo módulo `state`. Aplicação dos perfis de autopitch à seleção
  fica em `src/gui/piano_roll/state/autopitch.rs`, com os nomes históricos
  reexportados por `state`. Definição e consulta das escalas musicais ficam em
  `state/scale.rs`; quantização dependente de BPM e zoom fica em
  `state/quantize.rs`, com reexports pelo módulo `state`. `piano_roll/mod.rs`
  enumera os nomes públicos preservados em vez de reexportar todos os itens
  dos módulos `grid` e `state` sem distinção.
  Tags de letra, badges de fonema/modo e controles de vibrato são desenhados em
  `src/gui/piano_roll/note_overlays.rs`; consulta e substituição do fonema ativo
  ficam em `src/gui/piano_roll/lyric.rs`, mantendo os reexports compatíveis.
  Pointer continua responsável por editar notas e PitchDraw pelos pontos de
  pitch. As notas
  secundárias são desenhadas como camada não interativa em
  `src/gui/piano_roll/ghost_notes.rs`; o desenho e a interação do envelope de
  cinco pontos no painel de parâmetros ficam em
  `src/gui/piano_roll/parameter_drawer/envelope.rs`. Na Copaíba, a tela para bancos KFV 2 fica
  em `src/copaiba/gui/native_ui.rs`, o assistente de criação em
  `src/copaiba/gui/wizard.rs`, e a migração assistida de bancos clássicos em
  `src/copaiba/gui/migration.rs`. Prévia de unidades e frases fica em
  `native_preview.rs`; leitura e reprodução de WAV em `native_assets.rs`; criação
  e empacotamento de rascunhos em `native_authoring.rs`; bancos clássicos e
  aliases em `legacy_bank.rs`; o editor visual clássico em `legacy_editor.rs`;
  a barra superior em `toolbar.rs`; a tela inicial sem banco aberto em
  `welcome.rs`.
  A composição da tela e o estado compartilhado
  permanecem em `gui.rs`; criação, consulta e planejamento ficam em
  `native_project.rs`; edição de unidades, gravações e undo/redo ficam em
  `native_project/authoring.rs`;
  carregamento,
  validação e empacotamento KFV 2 ficam em
  `src/copaiba/native_project/storage.rs`.
  No arranjo, o cache de previews e o cálculo de nível dos clipes de áudio ficam
  em `src/gui/arrangement/waveform.rs`; o remapeamento de trilhas e as conversões
  dos gestos de navegação ficam em `src/gui/arrangement/operations.rs`.
- `src/renderer/`, `src/audio/` e `src/drivers/` dividem planejamento,
  renderização, reprodução e integração com engines. O render de projeto e
  seleção de trilhas ficam em `renderer/execution.rs`; a resolução de alvos e
  contexto para prévias isoladas fica em `renderer/preview.rs`; entrega e
  retomada de chunks ficam em `renderer/progressive.rs`; a mistura de trilhas,
  acumuladores SIMD e configuração do limiter ficam em `renderer/mix.rs`; a
  preparação do cache WAV e o tratamento de fonemas sem amostra ficam em
  `renderer/track/preload.rs`; leitura, conversão e escrita WAV ficam em
  `renderer/track/wav_io.rs`; a junção sequencial, concatenação de frases e
  finalização de áudio ficam em `renderer/track/merge.rs`; persistência e
  validação por hash do cache de resamplers ficam em
  `renderer/resampler_cache/persistent.rs`; o cálculo da identidade das
  entradas fica em `renderer/resampler_cache/key.rs`; o backend de desktop fica
  em `audio/player/desktop.rs`; o backend Web Audio fica em
  `audio/player/web_audio.rs`; eventos, estados, comandos e handle do scheduler
  ficam em `audio/player/scheduler.rs`. As APIs continuam reexportadas por
  `audio/player.rs`.
  O parsing, merge e
  edição de flags UTAU ficam em `drivers/resampler_flags.rs`; os cinco helpers
  e `UtauFlagToken` são reexportados nominalmente em `resampler_driver.rs` para
  manter os caminhos existentes. O catálogo de
  nomes, aliases e descoberta de executáveis fica em
  `drivers/resampler_catalog.rs`, ainda reexportado por `resampler_driver.rs`;
  o protocolo clássico compartilhado, incluindo argumentos, preparação de flags,
  identidade de cache e leitura do WAV de saída, fica em
  `drivers/resampler/classic.rs` e mantém os caminhos públicos de diagnóstico;
  o binding e a validação do resampler WORLD nativo ficam em
  `drivers/resampler/native_world.rs`, mantendo os erros sem fallback e os
  nomes públicos antigos; a integração com o processo clássico MacRes fica em
  `drivers/resampler/macres.rs`, usando o protocolo comum; os perfis externos
  e a configuração do servidor Hifisampler ficam em
  `drivers/resampler/external.rs`; a implementação SOLA nativa fica em
  `drivers/resampler/native.rs`;
  o catálogo de wavtools fica em `drivers/wavtool_catalog.rs` e é reexportado
  por `wavtool_driver.rs`; Andromeda e Galapagos ficam em
  `drivers/wavtool/native.rs`, e o driver Yawu fica em
  `drivers/wavtool/yawu.rs`; o adapter de executáveis genéricos fica em
  `drivers/wavtool/external.rs`, que também concatena frases por processo;
  leitura do WAV produzido fica em `drivers/wavtool/io.rs`. Os tipos e a função
  de concatenação seguem reexportados pelo módulo antigo;
  a API histórica do limiter permanece encaminhada pelo renderer. A identidade
  de cache, flags, aliases e diagnósticos explícitos do Tainah WORLD são
  contratos observáveis.
- `src/formats/`, `src/project/`, `src/oto/` e `src/voicebank/` definem os
  formatos, modelos, bancos e operações de projeto. Os modelos de pitch bend,
  curvas de expressão e respectivos defaults ficam em
  `src/project/model/expression.rs`; `model.rs` os reexporta para manter os
  caminhos públicos existentes por uma lista explícita de tipos. Trilha, parte
  vocal e parte de áudio ficam em `src/project/model/parts.rs`, igualmente
  reexportadas por `model.rs` com nomes enumerados.
  Normalização, mudança de tempo e recálculo explícito dos dados importados
  ficam em `src/project/model/operations.rs`, mantendo os métodos de `UProject`.
  Os tipos serializados do contrato KFV 2 ficam em
  `src/voicebank/native/schema.rs` e são reexportados nominalmente por
  `native.rs`; as regras de manifestos, perfis fonéticos e arquivos de unidades
  ficam em `native/validation.rs`; leitura de pastas e pacotes fica em
  `native/loader.rs`; hash e referência de identidade ficam em
  `native/identity.rs`; contagem de cobertura fica em `native/coverage.rs`.
  Os tipos e métodos de banco continuam acessíveis por `native.rs`.
  Na migração clássica, a leitura recursiva de `oto.ini`, o tratamento de aliases
  duplicados e a conversão de marcas de tempo em quadros ficam em
  `src/voicebank/migration/source.rs`; a prévia e a criação atômica da cópia
  KFV 2 permanecem em `migration.rs`.
  A leitura de aliases clássicos e os relatórios de resolução ficam em
  `src/oto/voicebank/lookup.rs`; a descoberta de metadados e carregamento
  clássico ficam em `loader.rs`; a varredura recursiva de `oto.ini`, com
  precedência da raiz, fica em `scanner.rs`; a extração segura de pacotes `.kfv`
  fica em `archive.rs`, e a apresentação compacta dos diagnósticos de `oto.ini`
  em `diagnostics.rs`. `voicebank.rs` mantém os tipos do banco clássico.

## Porte independente da análise estática do UTAU

Os exports Ghidra em `/Users/xiao/utau_legacy` são pseudocódigo de referência,
não fonte original ou biblioteca importável. As implementações abaixo são
independentes e mantêm explicitamente as lacunas ainda não confirmadas.

### Pitch e sinc

- O modelo do projeto guarda `UPitchBendPoint` em milissegundos relativos à
  nota e cents relativos ao pitch base. `src/renderer/track.rs` forma os pontos
  efetivos e `encode_utau_base64_pitch` os serializa para resamplers clássicos.
- `src/dsp/utau_pitch_curve.rs` decodifica pares Base64 assinados de 12 bits e
  repetições `#n#`; o primeiro caractere carrega os seis bits altos, como
  confirmado no pseudocódigo e no encoder Kamafeu. O adapter exige que o
  chamador forneça o passo em milissegundos. A relação entre `P/Q`, o passo
  atual do encoder e a interpolação do resampler continua aberta. Um teste de
  integração no driver clássico valida a curva que o Kamafeu envia no argumento
  de CLI e a decodifica de volta em cents/pontos; essa verificação confirma o
  contrato do codec e da fronteira do driver, não equivalência do áudio.
- `src/dsp/utau_sinc.rs` recebe PCM `i16`, posição em índices de amostra, escala e
  stride em elementos PCM. Mantém a fórmula analisada isolada do
  `WindowedSincResampler`, que usa `f32`, janela Lanczos e normalização pelos
  pesos. O arredondamento dos limites e a acumulação x87 têm aproximações
  documentadas junto à função.

### Wavtool e envelope

O contrato numérico analisado do wavtool clássico é
`offset length p1 p2 p3 v1 v2 v3 v4 ovr p4 p5 v5`. O adapter do Kamafeu
monta e testa essa ordem em `src/drivers/wavtool/external.rs`:

| Parâmetro CLI | Campo enviado pelo Kamafeu |
| --- | --- |
| `offset`, `length` | `WavtoolArgs.skip_over_ms`, `duration_ms` |
| `p1..p3`, `v1..v4` | Campos homônimos de `WavtoolArgs.envelope` |
| `ovr` | `WavtoolArgs.overlap_ms` |
| `p4`, `p5`, `v5` | Campos homônimos de `WavtoolArgs.envelope` |

Em falha de execução externa, o driver inclui no erro os 13 valores numéricos
formatados como foram enviados, com os nomes da ordem CLI acima. Isso ajuda a
diagnosticar diferenças de argumentos entre wavtools sem alterar os valores.

O contrato CLI não contém `pre`. `sample_time_zero_ms` ancora o envelope apenas
nos drivers internos. O renderer converte `phoneme_envelope` para os campos de
duração com esta transformação de índices: `p1=0`, `p2=x1-x0`, `p3=x4-x3`,
`p4=0`, `p5=x2-x1`; os níveis usam `y0,y1,y3,y4,y2`, convertidos para
percentuais. Isso descreve a implementação Kamafeu, sem afirmar equivalência
geométrica ou sonora com UTAU.

Galapagos/Yawu usam interpolação linear; Andromeda usa curva cosseno. A curva
de ganho do wavtool UTAU e o mapeamento completo de `pre` do batch para a CLI
continuam desconhecidos, então os modos atuais seguem distintos e selecionáveis.

### Piano roll

O Kamafeu usa milissegundos, pitch MIDI fracionário, `px_per_ms` e `row_height`.
`src/gui/piano_roll/state.rs` contém as conversões compartilhadas de tempo/pitch
para tela e suas inversas; desenho e edição da curva de pitch usam essas
funções. A análise estática do UTAU ainda não fecha escala em ticks, snap e
gestos, portanto esta é uma consolidação do contrato interno Kamafeu, não uma
reprodução das regras do UTAU.

## Regras para extrações

Extrações devem manter formatos salvos, argumentos CLI, respostas JSON e APIs
públicas. Reexports podem servir de compatibilidade durante a migração. Cada
etapa deve ter diff revisável e validação focada; builds WASM e publicação ficam
fora das etapas nativas de refatoração.

## Lints

As supressões globais de lint foram reduzidas. Exceções de função extensa
recebem `too_many_arguments` na própria função; a supressão WASM para caminhos
de desktop inativos está documentada em `lib.rs`. Reexports globais permanecem como superfície pública de compatibilidade; cada
remoção futura exige verificar os usos internos e a compatibilidade desejada.
