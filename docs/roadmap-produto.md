# Roteiro para um Kamafeu Studio poderoso

Este roteiro prioriza criação musical confiável. Cada etapa só é considerada
pronta quando pode ser usada em projeto real, exportada e verificada; uma tela
bonita sem dados corretos não fecha uma entrega.

Para a comparação de referências e a justificativa da ordem das próximas
entregas, consulte a [auditoria comparativa de evolução](upstream-improvement-audit.md).

## Fundamentos já entregues

- Editor de notas, pitch, vibrato, envelopes UTAU de cinco pontos, fonemas,
  letras em lote, Auto-Pitch, humanização, legato, quantização e correção de
  sobreposições.
- Compatibilidade de projeto APS, UST, USTX, MIDI, VSQX, SVP, UFData e JSON,
  com auditoria pós-conversão para identificar redução de dados.
- Renderização WAV, FLAC e RAW, diagnóstico de áudio, cache e validação de
  projetos e voicebanks UTAU/DiffSinger.
- Autosave recuperável em APS no desktop, com navegador de snapshots que abre
  uma cópia segura para salvar sob outro nome.
- Marcadores e compasso persistentes: régua, grade, expressões, CLI e auditoria
  de conversão usam o mesmo dado musical.
- Operação por GUI, CLI, automação JSON Lines e MCP local; extensões WASM são
  verificadas sem acesso implícito a arquivos, rede ou processos.
- Prévia progressiva publica chunks numerados, com intervalo temporal e API de
  retry isolado preservando contexto fonético e o estado final.
- Proveniência de render com hashes de conteúdo, identidade de engines, flags,
  artefatos e diagnósticos determinísticos de timing por fonema.
- APS v3 com migrações determinísticas, validação não destrutiva e persistência
  de alias, timing, envelope, vibrato e portamento por fonema.
- Auditoria de conversão compara assinaturas de overrides por fonema e denuncia
  perdas em formatos que não possuem representação equivalente.
- SDK WASM com ABI de parâmetros e buffers, limites de sandbox, execução
  explícita por CLI/automação, diagnósticos estruturados de descoberta e
  exemplos de contrato.
- Inspeção de sidecars por `provenance-info`, sem reabrir ou re-renderizar o
  projeto, incluindo contagem e dados de timing por fonema.
- Diff semântico de revisões via `diff`, com índices de notas e partes de áudio
  alteradas para alimentar comparação visual e restauração seletiva.
- Restauração seletiva validada de notas, partes de áudio, marcadores e seções
  na camada de domínio, sempre sobre cópia, com confirmação e undo na GUI.

## Próximas entregas, na ordem de impacto

### 1. Confiabilidade do projeto

- Seções persistentes no projeto, além de comparação visual de versões.
- Comparação visual de versões salvas e restauração seletiva de partes do projeto
  na GUI, consumindo o diff semântico já disponível na camada de domínio. A
  restauração de notas, áudio, marcadores e seções com confirmação e undo já
  está disponível no navegador de snapshots.
- Diagnóstico com correções sugeridas e confirmação antes de qualquer mudança
  destrutiva. `ProjectIssue::suggestion()` já alimenta GUI, JSON, automação e
  `validate-project` textual sem aplicar reparos implicitamente.
- Testes de ida e volta por formato, incluindo letras, pitch, expressões,
  envelopes, vibrato, áudio e múltiplas faixas.

**Critério de saída:** um projeto importado pode ser inspecionado, corrigido,
convertido e renderizado sem perda silenciosa.

### 2. Fluxo vocal profissional

- Editor de fonemas por nota e por transição, com prévia do alias resolvido e
  escopo explícito para envelope, vibrato, portamento e timing por fonema.
- Curvas de expressão por trecho, não apenas valores por nota.
- Ferramentas de timing para consoante, preutterance, overlap e crossfade com
  visualização no piano roll, incluindo exibição dos diagnósticos persistidos
  no manifesto de render.
- Pré-escuta de trechos e renderização em segundo plano cancelável.

**Critério de saída:** uma passagem vocal pode ser afinada, articulada e
revisada sem depender de edição manual de arquivos `oto.ini`.

### 3. Arranjo e mixagem

- Marcadores de arranjo, grupos/buses, sends e automação de volume/pan por
faixa.
- Congelamento de faixa, stems e exportação de lote.
- Medidores de pico/LUFS e avisos de clipping antes da exportação final.

**Critério de saída:** uma música multifaixa pode sair em mix estéreo e stems
reproduzíveis a partir do mesmo projeto.

### 4. Ecossistema aberto

- ABI WASM estável para formatos, fonemizadores, efeitos e motores.
- Catálogo de extensões com compatibilidade de versão, permissões explícitas e
testes de contrato.
- Presets compartilháveis para voz, expressão, pitch e cadeia de efeitos.

**Critério de saída:** extensões de terceiros ampliam o Studio sem comprometer
segurança ou compatibilidade do projeto.

### 5. Plataformas e acessibilidade

- Paridade funcional documentada entre desktop e WebAssembly.
- Interface de toque para Android/iOS, com alvos maiores e arquivos acessíveis
pelos seletores nativos.
- Navegação total por teclado, leitor de tela e contrastes verificáveis.

**Critério de saída:** o mesmo projeto abre de forma previsível nos destinos
suportados, com limitações declaradas em vez de falhas silenciosas.

## Portões de qualidade permanentes

1. `cargo test --lib` e `cargo test --bin kamafeu` passam.
2. O alvo `wasm32-unknown-unknown --no-default-features` compila.
3. Conversão registra diferenças e CI pode usar `convert --fail-on-loss`.
4. Operações automatizadas nunca sobrescrevem o projeto de entrada: exigem um
   caminho de saída explícito.
5. Toda funcionalidade visual que altera música possui undo, estado de projeto
   persistente e pelo menos um teste de comportamento quando aplicável.

### Matriz de paridade verificável

O comando `kamafeu platform-info --json` é a fonte executável da matriz de
plataformas. Ele informa, para cada capacidade, se ela está disponível em
desktop, Android e WASM; testes no módulo de extensões protegem os limites
que não podem ser alterados silenciosamente. Em resumo:

| Capacidade | Desktop | Android | WASM |
| --- | --- | --- | --- |
| Formatos de projeto | Sim | Sim | Sim |
| Render UTAU nativo | Sim | Sim | Sim |
| Processos externos/Wine | Sim | Não | Não |
| DiffSinger ONNX local | Sim | Não | Não |
| Extensões WASM | Sim | Sim | Sim |
| Snapshots locais | Sim | Sim | Não |

As limitações devem ser apresentadas antes da ação na interface; a matriz não
autoriza fallback silencioso para outro engine.
