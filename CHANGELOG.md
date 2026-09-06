# CHANGELOG - Kamafeu Studio

Todas as mudanças notáveis deste projeto estão documentadas neste arquivo.

O formato é baseado no [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/),
e este projeto adere ao [Versionamento Semântico](https://semver.org/lang/pt-BR/).

---

## [0.0.4-hotfix.1] - 2026-09-06

### Correções de inicialização
- **Ícone da janela no Linux/X11**: Kamafeu Studio e Copaiba agora reduzem o ícone para até 128 × 128 pixels antes de enviá-lo ao sistema de janelas. Isso evita a requisição de aproximadamente 16 MiB da imagem original de 2048 × 2048 pixels, que causava `MaximumRequestLengthExceeded` durante a criação da janela, apresentado como `NoGlutinConfigs`. Um teste verifica o tamanho da requisição do ícone empacotado.

### Documentação e manutenção
- README reorganizado com primeiros passos, compilação, comandos e atalhos conferidos no código.
- Aplicada a formatação Rust pendente para atender à verificação da CI.
- Tipos `f32` explícitos nos valores de desenho da interface para compatibilidade com o compilador Rust usado pelo Actions.

---

## [0.0.4-hotfix] - 2026-09-06

### Correções (Hotfix)
- **Compatibilidade Linux (X11 / Linux Mint)**:
  - Adicionadas as features explícitas `x11`, `wayland` e `rwh_06` no crate `winit` e restaurado suporte a `x11` no `eframe` para alvos desktop.
  - Corrige a falha de inicialização `WinitEventLoop(OsError("neither WAYLAND_DISPLAY nor WAYLAND_SOCKET is set; note: enable the winit/x11 feature to support X11"))` em ambientes baseados puramente em X11 (Linux Mint Cinnamon/MATE/Xfce, Debian, Xorg).

---

## [Não lançado] - 2026-09-02

### Discord Rich Presence
- **Presença contextual e assets oficiais**: o status no Discord agora informa projeto, parte vocal ativa, notas, seleção, cantor, BPM, reprodução e progresso de exportação WAV. Adicionados botões para o repositório e downloads, atualização apenas quando o estado muda e assets de 1024 × 1024 com instruções de upload em `assets/discord/`.

### Identidade e Marca
- **Rebranding oficial para Kamafeu Studio**: Atualização completa de nomenclatura no cabeçalho das janelas (`ViewportBuilder::with_title`), barra de menus, diálogos nativos, painel de status LED, documentação (`README.md`, `CHANGELOG.md`, `Cargo.toml`) e telemetria de presença via Discord Rich Presence (`discord_rpc.rs`).

### Interoperabilidade e Compatibilidade com Wine (macOS & Linux)
- **Detecção Automática e Otimização do Subsistema Wine**:
  - Implementado escaneamento dinâmico em tempo de execução para executáveis Win32/Win64 (`.exe` em formato Portable Executable) nos sistemas operacionais baseados em Unix (macOS e Linux).
  - Resolução automática de binários em caminhos de sistema (`/usr/local/bin/wine`, `/opt/homebrew/bin/wine`, `/usr/bin/wine`, `/opt/wine/bin/wine64`, etc.).
  - **Tradução Automática de Caminhos (`to_wine_windows_path`)**: Caminhos POSIX de entrada e saída (`/Users/victor/...` e `/var/folders/...`) agora são traduzidos transparentemente para o formato Windows (`Z:\Users\victor\...` e `Z:\var\folders\...` com contrabarras `\`). Isso impede que os analisadores de linha de comando padrão do Windows (MSVCRT/MinGW) confundam barras `/` com argumentos ou switches (`/U`, `/v`), eliminando falhas de execução em resamplers como `TIPS.exe`, `moresampler.exe` e `ChopSampler.exe`.
  - **Modo CLI Headless Otimizado**: Injeção automática das variáveis de ambiente `DISPLAY=""` (evitando inicialização demorada de janelas e subsistemas Vulkan/MoltenVK no macOS para ferramentas em segundo plano) e `LANG="ja_JP.utf8"`.
  - **Compatibilidade Automática com `moresampler.exe`**: Criação ou atualização dinâmica do arquivo `moreconfig.txt` no diretório do binário com `resampler-compatibility on`.
  - **Busca Abrangente de Executáveis**: Adicionados múltiplos diretórios de descoberta automática (`std::env::current_exe`, `Documents/kamafeu/resamplers`, `Downloads`, `Library/Application Support/OpenUtau/Resamplers`, etc.).

### Arquitetura de Interface e Máquina de Estados da Edição de Notas
- **Desacoplamento de Estados da Tecla `Enter` na Edição de Lírica**:
  - Correção de conflito de despacho de eventos de teclado que revertia a edição ao pressionar `Enter`.
  - Implementada máquina de estados com transição determinística:
    - *Estado Ocioso (sem foco em edição)*: Pressionar `Enter` com uma nota selecionada inicializa o buffer de lírica (`state.lyric_buffer`), ativa a flag de seleção total de caracteres (`lyric_needs_select_all = true`) e foca o `TextEdit` inline.
    - *Estado de Edição Ativo*: Pressionar `Enter` finaliza imediatamente o ciclo de foco (`text_resp.lost_focus()`), confirma as alterações no grafo de notas do projeto (`commit_lyric_edit`), limpa os caches de fonemização dependentes (`phoneme_cache_hash = 0`), registra ponto no histórico de undo/redo (`Ctrl+Z`) e aciona a síntese em tempo real.

### Engenharia DSP: Pipeline de Transições, Resampler, Wavtool e Concatenação
- **Cache content-addressed de fragmentos do resampler**:
  - Resultados são reutilizados em RAM e em `~/Library/Caches/kamafeu/resampler-v1`, evitando relançar TIPS/Wine para fonemas idênticos a cada prévia ou exportação.
  - A chave inclui amostra e metadados de análise (`.frq`, `.pmk`, `.mrq` e equivalentes), parâmetros OTO, pitch/curva, duração, flags, tempo e versão do executável.
  - Pedidos simultâneos idênticos são coalescidos em uma única execução e o limite de threads configurado na interface agora controla prévia e exportação.
- **SOLA adaptativo para canto**:
  - Substituída a estimativa global por análise YIN com refinamento subamostral e rejeição de material aperiódico.
  - As marcas de pitch agora mantêm coerência de fase por correlação local; os grãos usam interpolação cúbica, ganho de overlap e correção RMS limitada.
  - Ataque e cauda do fonema são preservados fora do loop, com crossfade equal-power de 5 ms na fronteira consoante/vogal.
  - Regiões não vozeadas usam WSOLA automaticamente, e o antigo modo incorretamente chamado `Phase Vocoder` foi renomeado para `SOLA Híbrido`.
- **Restauração do Alinhamento de Fase (*Phase-Aligned Crossfade*) e Eliminação de Aspereza**:
  - Restaurada em `TrackRenderer::mix_phase_aligned` a busca por correlação cruzada normalizada com janela de busca adaptativa de `lag` (-search..=search).
  - Alinha os picos e vales das frequências fundamentais entre fonemas vizinhos na região de sobreposição, eliminando o cancelamento destrutivo de fase, filtragem em pente (*comb filtering*) e artefatos ásperos/robóticos.
- **Transparência na Execução de Motores**:
  - Cada fonema renderizado agora registra explicitamente o motor em execução no log do console (`[Resampler] Motor: '<Nome>'`), facilitando o diagnóstico em tempo real.
- **Análise e Alinhamento com a Arquitetura do OpenUtau (`OpenUtau-master`)**:
  - Eliminação de gargalos e descontinuidades de fase em transições críticas de dífonos e trifonemas fricativos e africados (ex.: transições `ch i` + `i s`).
  - Refatoração do módulo interno de junção (`WavtoolDriver` nativo):
    - Implementação de janelamento de crossfade equal-power com curvas de cosseno elevado (*raised-cosine windowing*) em substituição a interpolações estritamente lineares.
    - Normalização estrita de energia RMS na sobreposição de envelopes de transição fonética para prevenir clipping ou quedas abruptas de amplitude espectral (*notching*).
    - Preservação da coerência de fase relativa entre harmônicos na junção das janelas de pitch (*WSOLA Phase Matching*).

### Formatos de Intercâmbio e Ecossistema UtaFormatix
- **Exportação e Compatibilidade com Synthesizer V (`.svp`)**:
  - Implementado serializador `SvpFormat::save_file` gerando árvores JSON estruturadas conforme a especificação do Synthesizer V (versão de projeto 100).
  - Mapeamento bidirecional de faixas, grupos de notas (`tracks.groups`), parâmetros temporais em ticks (escala standard 1470 ticks/semínima), durações, alturas MIDI e offsets de tempo.
- **Exportação e Compatibilidade com Vocaloid Sequence XML (`.vsqx` / VSQ4)**:
  - Implementado gerador de documentos XML `VsqxFormat::save_file` com schema estruturado (`VSQ4` para Vocaloid 4 / UtaFormatix).
  - Serialização de cabeçalhos de tempo, compassos, andamentos (`tempoList`), faixas com blocos `<musicalPart>`, notas `<note>` com durações em ticks e fonemas SAMPA mapeados.
- **Unificação de Diálogos de Abertura, Importação e Exportação**:
  - Os filtros do seletor de arquivos (`rfd::FileDialog`) passaram a aceitar todas as extensões em formatos maiúsculos e minúsculos: `.aps`, `.ustx`, `.ust`, `.ufdata`, `.svp`, `.vsqx`, `.vsq`, `.mid`, `.midi`, `.json`.
  - Criação de entradas dedicadas nos submenus `📥 Importar` e `📤 Exportar` da barra superior (`menu_bar.rs`) para `.ufdata`, `.svp` e `.vsqx`.

### Edição Direta e Cirúrgica de Fonemas na Régua Inferior
- **Editor Inline via Clique Duplo em Badges**:
  - Adicionada detecção de duplo clique sobre as pílulas de fonemas na régua de transições (`phoneme_ruler.rs`).
  - **Subfonemas Compostos**: Ao dar duplo clique em um subfonema específico (ex.: `[p o]` em uma nota composta por `p o.k a`), abre-se um editor inline com fundo `rgb(45, 30, 75)` e borda dourada. Modificar o segmento para `p oh` altera cirurgicamente apenas aquela posição da nota, gerando `p oh.k a` sem desbalancear as durações relativas previamente arrastadas.
  - **Fonemas Únicos**: Ao dar duplo clique em uma nota simples (ex.: `[- ka]`), o editor inline abre preenchido permitindo reescrever a lírica diretamente no rodapé.
  - Suporte completo a atalhos de confirmação (`Enter`), cancelamento (`Escape`) e commit ao desfocar (`click_outside`).

### Arquitetura Gráfica do Piano Roll: Pass 1 vs Pass 2
- **Eliminação de Sobreposição de Pitch sobre Líricas e Mini-Toolbar**:
  - Reestruturação do pipeline de pintura (`egui::Painter`) dividindo o ciclo de renderização de notas em dois passos sequenciais estritos:
    - *Passo 1 (Base & Curvas)*: Desenho do corpo da nota (blobs estilo Melodyne), formas de onda internas, envelopes de volume, modulação senoidal de vibrato e curvas de pitch bend contínuas.
    - *Passo 2 (Primeiro Plano Absoluto)*: Execução diferida (`pending_lyric_tags` e `pending_phoneme_badges`) desenhando caixas de texto com fundo 100% opaco (`rgb(26, 18, 8)`) sobrepostas às curvas de pitch.
  - O texto da letra nunca mais é atravessado ou cortado pela linha de pitch, garantindo 100% de legibilidade e contraste cromático.
  - O fundo da mini-barra flutuante (`〰 Vib`, `🎚 Env`, `📈 Pitch`, `⚙ Prop`) foi tornado 100% opaco (`rgb(15, 12, 24)`), isolando-a contra trajetórias de notas adjacentes.

### Renderização Visual da Curva de Pitch e Micro-Nós de Controle
- **Translucidez e Suavidade da Curva de Pitch**:
  - Curva de pitch suavizada com opacidade calibrada para não poluir o piano roll:
    - *Modo Navegação/Reprodução*: Traço de `1.4px` com dourado suave translúcido `rgba(255, 215, 0, 110)` (~43% de opacidade).
    - *Modo de Edição (`PitchDraw`)*: Traço de `2.0px` com `rgba(255, 215, 60, 175)` (~68% de opacidade).
- **Refinamento e Redução dos Nós de Controle**:
  - Substituídos os círculos roxos maciços de raio `5.0px`/`7.0px` (até `14px` de diâmetro) com bordas brancas espessas por micro-nós elegantes padrão DAW profissional:
    - *Nó em Repouso*: Raio reduzido para `2.3px`, preenchido em ouro acetinado (`rgb(255, 220, 100)`) com contorno escuro ultrafino de `0.8px` (`rgba(20, 15, 28, 220)`).
    - *Nó sob o Mouse (`Hover`)*: Expansão fluida para `3.4px` acompanhada de halo ciano emissivo `rgba(0, 230, 255, 95)` e núcleo clareado.
    - *Nó em Arrasto (`Dragging`)*: Expansão para `3.8px` com halo dourado radiante `rgba(255, 215, 80, 85)`.
- **Algoritmo de Desagregação Visual e Culling de Densidade**:
  - Implementado filtro de densidade na renderização: pontos secundários espaçados por menos de `9.0px` na tela não acumulam esferas sobrepostas, eliminando o "efeito lagarta" em curvas desenhadas à mão ou vibratos com alta densidade temporal.
  - Nós de extremidade (início e fim da curva), nós sob o cursor do mouse e o nó ativo sob manipulação possuem prioridade irrestrita e são sempre renderizados.

### Sincronização Temporal no Modo Página (`AutoScrollMode::PageScroll`) e Bordas
- **Sincronização de Rolagem Prévia no Ciclo de Frame**:
  - O cálculo de salto de página de `AutoScrollMode::PageScroll` foi movido para o início da rotina de renderização (`piano_roll/mod.rs`), antes de calcular `timeline_scroll_x` e antes da invocação das réguas superior e inferior.
  - Elimina a defasagem temporal de 1 frame em que o piano roll saltava para a nova página mas a régua inferior permanecia na coordenada antiga.
- **Tolerância Ampliada e Ancoramento Dinâmico de Badges**:
  - Margem de culling horizontal aumentada de `120px` para `300px` em `phoneme_ruler.rs` e no piano roll (`extended_clip = visible_clip.expand2(300, 40)`).
  - Para notas que ultrapassam a margem esquerda da página visível, o badge de fonema (tanto em notas simples quanto subfonemas múltiplos) agora utiliza *clamping* dinâmico com linha de ancoragem conectora, mantendo o controle visível e interativo dentro da área visível da tela.
- **Preservação de Contexto Fonético VCV entre Chunks e Páginas**:
  - Refatorado `ProjectRenderer::preview_context_start` para incluir a nota imediatamente precedente (se a distância temporal for inferior a 400ms).
  - Garante que ao renderizar blocos progressivos de áudio durante o avanço do Modo Página, o fonemizador VCV preserve a vogal precedente da nota anterior, evitando que o fonema de entrada seja sintetizado incorretamente como fonema isolado (`- ka` em vez de `a ka`).

---

## [Não lançado] - 2026-08-31

### Segurança

- Removidas do `Cargo.toml` as credenciais de assinatura Android que estavam em texto puro.
- Build Android alterado para usar os secrets `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` e `ANDROID_KEY_PASSWORD`.
- Importação de `.kfv` protegida com limites de 10.000 entradas, 512 MiB por arquivo, 4 GiB no total e detecção de taxas de compressão suspeitas.
- Caminhos de WAV e imagens de voicebanks agora rejeitam caminhos absolutos, prefixos de plataforma e componentes `..`.
- Entradas inseguras vindas de `oto.ini` são ignoradas e configurações JSON inseguras retornam erro.
- Dependências vulneráveis atualizadas ou removidas do grafo ativo; `cargo audit` passou a fazer parte do CI.
- Recurso `accesskit` do `eframe 0.29` temporariamente desabilitado para remover uma cadeia Linux vulnerável baseada em `quick-xml`; deverá ser reativado após a futura migração do `egui/eframe`.

### Corrigido

- Corrigida a voz de “esquilo” reproduzida com o alias `k ae` do KYE no projeto `dancing_queen.aps`, tanto no TD-PSOLA quanto no SOLA/WSOLA.
- Regiões consonantais do OTO encurtadas para caber em fonemas breves agora usam WSOLA com preservação de tom; a reamostragem linear anterior alterava a velocidade e elevava o pitch das transições vocálicas contidas nessas regiões.
- Adicionado teste de regressão para a proporção real encontrada no KYE (`204 ms` de região consonantal reduzidos para aproximadamente `96 ms`) e validado o resultado no projeto real em B3.

### Edição de fonemas

- O autocomplete do `oto.ini` continua sugerindo aliases depois de `.`, `,` ou `;`: a busca considera apenas o fonema em edição e, ao escolher uma opção, preserva toda a sequência anterior.
- Notas com vários fonemas explícitos, separados por `.`, `;` ou `,`, agora exibem cada segmento individualmente na régua inferior, incluindo sua duração em milissegundos.
- As fronteiras douradas entre subfonemas podem ser arrastadas para redistribuir a duração sem alterar o tamanho total da nota; por exemplo, `k ae.ae n.` em 480 ms pode ser ajustado para `360 ms + 120 ms`.
- Toda a linha vertical entre dois subfonemas agora possui captura própria de redimensionamento, com cursor horizontal e realce visual; o arraste permanece ativo mesmo fora da área estreita do separador e não é mais necessário acertar o pequeno quadrado dourado no rodapé.
- As durações personalizadas são usadas pelo fonemizador/renderizador, acompanham o redimensionamento total da nota e são persistidas no arquivo `.aps` com compatibilidade para projetos antigos.
- O botão direito na nota da régua também restaura a divisão igual entre os subfonemas.

### Fade e crossfade por nota

- Cada nota agora possui controles próprios de `Fade-in`, `Fade-out` e `Crossfade` na aba **Nota**, além de predefinições suave, seca e automática.
- Crossfades individuais são desenhados diretamente sobre a nota como uma região em “X”, seguindo a referência visual de editores de áudio.
- O valor individual controla o overlap usado pelo renderer e pelo wavtool; em `0 ms`, permanece ativo o comportamento automático baseado no OTO e na configuração global.
- Fade-in, fade-out e crossfade são salvos no `.aps`, aplicam-se a múltiplas notas selecionadas e são normalizados ao carregar projetos.

### Outras correções

- `Ctrl/Cmd+Z` e `Ctrl/Cmd+Y` agora trabalham com uma transação por gesto: criação, movimento e redimensionamento guardam o estado anterior ao pressionar o mouse e confirmam uma única entrada ao soltar, eliminando estados parciais em que a nota apenas diminuía em vez de desaparecer.
- Reduzidas as pausas da interface ao confirmar letras e soltar edições: buscas sem distinção de caixa no `oto.ini` agora usam um índice construído uma única vez, sugestões são recalculadas apenas quando a consulta muda, ajustes de duração não invalidam o cache de aliases e o histórico deixa de desalocar sua memória a cada alteração.
- O editor inferior de envelopes agora apresenta todas as notas visíveis na mesma linha do tempo do piano roll, acompanhando zoom e rolagem horizontal.
- Curvas de cinco pontos podem ser editadas em qualquer nota sem fechar ou trocar o painel; clicar numa alça também seleciona a nota correspondente.
- Transições adjacentes exibem o crossfade como um “X”, inclusive quando usam overlap automático, e a alça na borda esquerda permite ajustar diretamente sua duração individual.
- Corrigida a barra horizontal do painel **Parâmetros / Expressões**, que voltava à altura anterior depois de ser arrastada; a altura final agora é capturada após o redimensionamento e permanece estável entre os frames.
- O editor ampliado de envelope recebeu a mesma correção de persistência de altura.
- O editor de envelope de cinco pontos deixou de ficar comprimido dentro da altura da nota: o botão `Env` agora abre um painel inferior amplo e redimensionável, com grade, valores em ms/%, alças maiores e edição horizontal/vertical precisa.
- As notas mantêm apenas uma prévia discreta da curva de volume, liberando o bloco para letra, waveform e demais informações.
- Notas longas deixam de reiniciar a amostra e repetir o ataque ao atravessar os blocos da prévia progressiva; cada bloco seguinte mantém o contexto desde o início da nota e recorta apenas a porção anterior já reproduzida.
- A trilha de waveform do piano roll voltou a permanecer visível: sua posição agora acompanha as notas presentes na área vertical exibida, em vez de depender da nota mais grave de todo o projeto.
- A prévia progressiva passa a anexar à waveform os blocos renderizados em segundo plano, permitindo que o desenho avance junto com a reprodução além dos dois primeiros segundos.
- Consultas fora do intervalo de áudio renderizado deixam de repetir indefinidamente o último pico da waveform.
- Corrigida a voz excessivamente aguda (efeito “esquilo”) do TD-PSOLA ao aplicar portamentos ou curvas leves de pitch.
- Removida a reconstrução LPC duplicada dos grãos sonoros do TD-PSOLA; os grãos completos agora preservam o envelope espectral sem amplificar artefatos de alta frequência.
- Adicionado teste de regressão que mede a frequência resultante de uma curva suave de `-20` a `+20` cents nos motores TD-PSOLA e SOLA.
- Corrigida a semântica de `cutoff` nos motores internos TD-PSOLA e SOLA/WSOLA: valores positivos removem tempo do fim do WAV e valores negativos definem o comprimento absoluto a partir do offset, conforme o formato UTAU.
- Aliases recortados de WAVs longos não reproduzem nem criam loops acidentais com fonemas posteriores do mesmo arquivo.
- O fallback do renderer e o marcador de corte final do Copaiba agora usam a mesma semântica dos resamplers externos.
- Adicionado teste de regressão com um alias curto armazenado no início de um WAV longo.
- Corrigido panic no parser de nomes de notas ao receber texto Unicode ou UTF-8 malformado para o formato esperado.
- Corrigida a conversão de tons USTX fora do intervalo MIDI: o valor agora é limitado antes da conversão para `u8`.
- BPM de projetos USTX agora é normalizado para valores finitos entre 20 e 999.
- Importação MIDI agora rejeita divisão temporal zero e eventos de tempo zero.
- Projetos USTX tipados também passam pela normalização de invariantes.
- Erros ao carregar e salvar a configuração global deixaram de ser descartados silenciosamente.
- Salvamento do Copaiba agora propaga falhas de `character.txt` e `prefix.map`.
- Arquivos de configuração são gravados por arquivo temporário, sincronizados e substituídos atomicamente.
- Falhas na inicialização Android por `eframe::run_native` agora são registradas.
- Teste VCV tornou-se autocontido e deixou de depender de caminhos `../../../` incompatíveis com o confinamento de voicebanks.

### Drivers e renderização

- O motor nativo ganhou quatro estratégias selecionáveis no painel: `SOLA Stretch`, `SOLA Loop`, `SOLA Spline` e `Phase Vocoder`, com avanços distintos para naturalidade, sustentação, suavização de trajetória e coerência de fase em notas longas.
- WORLD foi deliberadamente deixado fora desta etapa; a integração externa World4UTAU existente continua separada dos modos SOLA.
- Adicionada regressão que executa todos os modos internos e verifica duração exata e saída não silenciosa.
- A reprodução pela barra de espaço agora usa prévia progressiva: um primeiro bloco prioritário de dois segundos começa a tocar assim que fica pronto, enquanto blocos seguintes de quatro segundos são renderizados e enfileirados em segundo plano.
- O renderer ganhou seleção por intervalo temporal, evitando processar centenas de notas futuras antes de iniciar a reprodução.
- O player de áudio agora aceita blocos contíguos na fila ativa sem reiniciar o dispositivo ou interromper o bloco que já está tocando.
- Wavtools agora retornam `Result` e verificam criação da entrada, início do processo, código de saída e `stderr`.
- Falhas de wavtool são registradas e acionam fallback explícito para o processamento nativo.
- Resamplers e wavtools externos receberam timeout de 120 segundos.
- Cancelamento de renderização agora é propagado aos processos externos e encerra o processo filho.
- Captura de `stdout` e `stderr` limitada a 1 MiB por fluxo para evitar consumo ilimitado de memória.

### Copaiba Voicebank Toolkit

- Remoção de avatar agora apaga com segurança o arquivo exibido e impede restauração involuntária no frame seguinte.
- Waveforms anterior e seguinte passaram a ser armazenadas em cache, eliminando decodificação de WAV a cada frame.
- Exclusão e duplicação de aliases atualizam imediatamente seleção, áudio e previews.
- Zoom horizontal passou a afetar waveform, régua temporal e interação com marcadores.
- Renderização do envelope de waveform distribui todas as amostras pela largura sem perder o trecho final.
- Erros de abertura, salvamento, importação de imagem e remoção são apresentados na interface.
- Empacotamento ignora links simbólicos e reporta erros de leitura de diretório.
- Atalhos do editor deixam de ser bloqueados pelo simples foco em widgets não textuais.
- Arquivos `.kfv` aninhados são ignorados sem distinção entre maiúsculas e minúsculas.

### Qualidade e automação

- Projeto completo formatado com `cargo fmt --all`.
- CI endurecido com `cargo clippy --all-targets --all-features -- -D warnings`.
- CI agora instala e executa `cargo-audit`.
- Lockfile atualizado, incluindo correção de `webbrowser` para uma versão sem a vulnerabilidade conhecida.
- Adicionado teste de regressão para nomes de notas Unicode.
- Suíte atual: 74 testes aprovados, incluindo regressões dos quatro modos de stretch, do autocomplete de sequências fonéticas e da integridade de Undo/Redo; Clippy sem avisos e build release aprovado.
- Relatório técnico consolidado em `AUDITORIA_SOFTWARE.md`.

### Navegação Rápida com TAB, Escala de Interface (DPI) e Modularidade
- **Navegação com Tecla TAB e Shift+TAB nas Notas**:
  - Pressionar **TAB** ao editar uma nota confirma a letra e pula instantaneamente para a próxima nota para edição contínua e rápida de sílabas.
  - Pressionar **Shift+TAB** retrocede para a nota anterior.
  - Eliminação total do input lag: confirmação e comutação instantânea no mesmo frame de renderização.
- **Alças e Controles de Transições Aumentados (Phoneme Ruler)**:
  - Régua de fonemas e envelopes ampliada de 36px para 46px com alças visuais e botões interativos 50% maiores (com borda de alto contraste e área de clique de 20px).
- **Escala de Interface e Zoom DPI Ajustável (Menu Exibir -> Escala de Interface)**:
  - Controle dinâmico de DPI com perfis: `75% (Muito Compacto)`, `85% (Compacto)`, `90% (Espaçoso)`, `100% (Padrão)`, `110% (Ampliado)`, `125% (HiDPI)` e `150% (Muito Grande)`.
  - Preferência salva automaticamente no arquivo de configuração do usuário.
- **Painel de Multifaixas Modular / Colapsável**:
  - Opção no menu `Exibir -> Exibir Painel de Multifaixas / Arrangement` para ocultar/mostrar a trilha superior de faixas e liberar mais espaço para o Piano Roll.

### Suporte a G2P e Alternância entre Fonética Direta vs. Palavras (Grapheme-to-Phoneme)
- **Alternância Flexível G2P vs. Fonética Direta**:
  - **Modos G2P (`English G2P`, `Português G2P`)**: Permite escrever palavras completas diretamente na letra das notas (ex: `"can"`, `"sing"`, `"sol"`), convertendo automaticamente o texto em sequências fonéticas completas (ARPABET ou BRAPA/PT-BR).
  - **Modos de Fonética Direta (`English Arpasing`, `English VCCV`, `BRAPA CVC`, `PT CVVC`, `PT VCV`)**: Mantém a interpretação literal dos fonemas e dífonos digitados (ex: `"k ae"`, `"dh"`, `"so"`), sem conversão léxica.
- **Indicadores de Estado do Projeto e Título Dinâmico**:
  - Exibição de asterisco `*` no título da janela e pill estilizado `* nome.aps (Não salvo)` no topo quando houver alterações pendentes.

### Barra Avançada de Controle de Fonemas e Transições (Phoneme Envelope Ruler)
- **Barra de Fonemas Interativa Multiponto**: Localizada diretamente acima da janela de parâmetros, exibindo a subdivisão fonética em tempo real alinhada à timeline.
- **Controle Preciso de 3 Pontos de Transição**:
  - **Pré-emissão (Preutterance - Ciano)**: Alça de ataque que define a antecedência de articulação antes da nota musical.
  - **Overlap / Crossfade (Magenta/Rosa)**: Alça de cruzamento suave entre o fonema anterior e a nota atual.
  - **Fronteira Consoante vs. Vogal (Verde/Dourado)**: Alça de separação entre o corpo consonantal e o loop/sustentação da vogal.
- **Feedback Visual com Cores Distintas**: Distinção clara entre regiões de consoante (azul-petróleo) e sustentação de vogal (roxo/lavanda) com tooltips ao vivo indicando tempos absolutos e offsets.
- **Reset Rápido e Varredura com Botão Direito**:
  - **Clique com Botão Direito**: Restaura instantaneamente os pontos da nota sob o cursor para o padrão do `oto.ini`.
  - **Segurar e Arrastar com Botão Direito (Sweep Reset)**: Permite varrer várias notas em sequência para resetá-las em lote de forma fluida.

### Formato Nativo de Projeto Saturno (`.aps`)
- **Novo Formato Oficial de Projeto (`.aps` - Arquivo Projeto Saturno)**: Padrão oficial e prioritário do Kamafeu Studio para salvar e carregar projetos completos com metadados nativos, faixas, notas, curvas de afinação, portamentos e envelopes.
- **Interoperabilidade Completa**: Suporte transparente a carregar e salvar em `.aps`, `.ustx` (OpenUTAU), `.ust` (UTAU Sequence) e `.mid` / `.midi` (Standard MIDI).

### Modo Sem Fonemizador (Manual) e Sub-Fonemas Literais
- **Opção `Sem Fonemizador (Manual)`**: Modo padrão para novos projetos, permitindo total controle e digitação direta de qualquer alias do `oto.ini`.
- **Suporte a Múltiplos Fonemas na Mesma Nota com `.`, `;` e `,`**: Permite encadear múltiplos fonemas literais dentro da mesma nota (por exemplo, `m an. an d. d eh. eh l. l a.` ou `-k;k ae;ae n` ou `k ae, ae n`), dividindo o tempo de forma proporcional e consultando individualmente cada alias no Voicebank.
- **Continuidade Universal com `+`**: Suporte ao caractere `+` para extensão de notas contínuas em qualquer modo ou fonemizador.

### Pincel Específico de Pitch com Traçado Suave (Smooth Pitch Brush)
- **Nova Ferramenta de Pincel de Pitch (`Pitch (P)`)**: Ferramenta dedicada para traçado e modelagem de curvas de afinação vocais (pitch bend) com atalho rápido `P` (ou `B` para Brush).
- **Algoritmo de Suavização Gaussiana em Tempo Real (`smooth_pitch_points`)**:
  - Filtro gaussiano ponderado de 5-tap (`kernel: [0.061, 0.245, 0.388, 0.245, 0.061]`) com relaxamento passa-baixas de 2ª ordem.
  - Elimina trepidações da mão e ruídos angulares do mouse, gerando curvas de afinação orgânicas, suaves e naturais estilo Melodyne e Synthesizer V.
- **Preview Visual Luminoso ao Vivo (Live Stroke Preview)**: Traço neon dourado anti-aliased acompanhando o cursor do mouse fluidamente durante o desenho.
- **Fitting Cúbico com Curvas-S (`shape = "s"`) e Curvas-J (`shape = "j"`)**: Conversão inteligente dos pontos traçados em nós de pitch bend simplificados e contínuos sem degraus ou descontinuidades acústicas.
- **Sub-modos Especializados do Pincel**:
  - **`Suave` (Padrão)**: Desenho livre com suavização gaussiana ativa.
  - **`Amaciar` (Smooth)**: Passar o pincel sobre uma curva de pitch existente amacia ondulações bruscas e remove picos acentuados.
  - **`Reta` (Line)**: Glissandos lineares com interpolação geométrica precisa.
  - **`Vibrato`**: Pincel de modulação harmônica senoidal contínua.

---

### Tratamento de Cliques, Isolamento de Ferramentas e Exclusão Mútua
- **Retenção Perfeita de Seleção**: O evento de soltar o mouse (release) sobre notas ou menus contextuais não desseleciona mais a nota nem fecha opções ao clicar na grade de fundo.
- **Isolamento Total das Alças de Envelope de Volume**:
  - Manipular a primeira bolinha (`p1`) ou a última bolinha (`p5`) do envelope **não redimensiona nem move mais a nota**.
  - Exclusão mútua rigorosa: o foco do cursor e da interação fica restrito aos nós do envelope de volume (`p1..p5`, `v1..v5`).
- **Isolamento das Âncoras de Pitch Bend**: Edição e arraste de âncoras de afinação não disparam redimensionamento ou movimento acidental da nota.
- **Matriz de Exceções e Comportamentos entre Ferramentas**:
  - **`Pointer (V)`**: Seleciona, arrasta e redimensiona notas existentes; clicar no vazio move a playhead; arrastar no vazio faz seleção em caixa (marquee). Nunca duplica notas por engano.
  - **`Pencil (N)`**: Clicar em nota existente seleciona/move/redimensiona (sem duplicar sobreposta); clicar e arrastar no espaço vazio cria uma nova nota.
  - **`Pitch (P)`**: Focado exclusivamente no traçado e modelagem de afinação (não cria notas nem move o cursor de reprodução ao clicar no vazio).
  - **`Eraser (E)`**: Apaga a nota clicada com 1 toque.

---

### Desfazer e Refazer (Undo/Redo) e Histórico em Tempo Real
- **Registro de Snapshot Antecipado**: `on_before_change()` chamado no momento exato do clique inicial (`primary_pressed`) de qualquer ação (arrasto de notas, nós de envelopes, traçado de pitch bend, criação com lápis, deleções e presets).
- **Atalhos Globais Confiáveis**: Suporte unificado a `Ctrl+Z` / `Cmd+Z` (Desfazer) e `Ctrl+Y` / `Cmd+Shift+Z` / `Ctrl+Shift+Z` / `Cmd+Y` (Refazer).
- **Feedback Visual na Barra de Status**: Notificações instantâneas (`Desfeito (Undo)` / `Refeito (Redo)`) ao reverter ou avançar estados.

---

### Waveform de Áudio Ultra HD com Alinhamento Temporal Pixel-Perfect
- **Trilha de Forma de Onda no Piano Roll**: Visualização contínua da waveform de áudio sintetizada logo abaixo das notas na grade, com preenchimento anti-aliased e cristas em alta definição.
- **Alinhamento Temporal Absoluto**:
  - Correção matemática da conversão pixel-para-tempo: eliminada a soma espúria de `timeline_scroll_x` no interior do `ScrollArea`.
  - A forma de onda do áudio sintetizado agora fica **100% alinhada com as notas e compassos da grade**, independente de onde a playhead esteja e independente da rolagem horizontal da tela.
- **Suporte Multicanal e Estéreo**: Processamento de áudio estéreo interleaved e mono com conversão precisa de picos a cada `1.5ms`.

---

### Mini Floating Action Toolbar e Despoluição das Notas
- **Corpo da Nota Limpo e Elegante**: Bolas de envelope de volume e âncoras de pitch bend foram despoluídas do corpo da nota na visualização padrão.
- **Mini Barra de Ação Flutuante**: Ao selecionar uma nota, uma barra contextual com 4 mini botões é exibida logo acima da nota:
  - **`Vib`**: Popover com presets de vibrato a 1 clique (Pop, Drama, Lento, Rápido, Desligar) e sliders finos de controle (comprimento, profundidade, período, fade in/out, shift e fase).
  - **`Env`**: Alternador (toggle) para exibir alças de envelope de volume sob demanda.
  - **`Pitch`**: Alternador para ativação do modo de desenho e âncoras de afinação.
  - **`Prop`**: Abre a janela de propriedades e expressões completas da nota.

---

### Gerenciador de Cantores do OpenUtau (Singers) e Galeria de Cantores
- **Registro de Pastas de Cantores**: Suporte a registrar a pasta de voicebanks padrão do OpenUtau (`~/Library/Application Support/OpenUtau/Singers` ou caminhos personalizados) com persistência em `~/.config/kamafeu/kamafeu_config.json`.
- **Escaner Recursivo de Voicebanks**: Identificação automática de cantores em subpastas com leitura de `oto.ini`, `character.txt`, `character.yaml` e avatares.
- **Janela de Galeria de Cantores (Singers Gallery)**:
  - Grade visual responsiva (Grid View) com cards de cantores.
  - Imagens dos cantores preservando taxa de proporção (aspect ratio) sem distorções.
  - Barra de busca instantânea por nome ou autor do cantor.
  - Botão `Carregar este Cantor` com 1 clique.
  - Seleção e reprodução rápida de prévias vocais.

---

### Otimizações de Performance e Gerenciamento de Memória
- **Renderização Multithread Paralela**: Síntese de áudio multifaixa em paralelo utilizando Rayon (`par_iter`), sem travar a thread da interface gráfica.
- **Cache de Arquivos WAV em Memória (`Arc<Vec<f32>>`)**: Leitura única de arquivos de áudio do voicebank com reaproveitamento instantâneo em memória RAM (Zero Disk I/O em repetições).
- **Truncamento de Silêncio nos Buffers**: Redução de alocações excessivas de memória em buffers de áudio com silêncio final.
- **Repaint Inteligente a 60 FPS**: Renderização contínua a 60 FPS ativada exclusivamente durante a reprodução de áudio ou scrubbing de régua, reduzindo o uso de CPU em repouso a quase 0%.
- **Playhead Fina e Fluida**: Cursor de reprodução com espessura de `1.0px` e crista luminosa com transições suaves pelo Piano Roll.

---

### Barra de Menus Superior Completa e Painéis Profissionais
- **Menu Superior Estilo DAW Profissional**:
  - **Arquivo**: `Novo Projeto (Cmd+N)`, `Abrir Projeto... (Cmd+O)`, `Salvar Projeto (Cmd+S)`, `Salvar Como...`, `Carregar Voicebank...`, `Exportar Áudio WAV... (Cmd+E)`, `Fechar`.
  - **Editar**: `Desfazer (Cmd+Z)`, `Refazer (Cmd+Shift+Z)`, `Recortar (Cmd+X)`, `Copiar (Cmd+C)`, `Colar (Cmd+V)`, `Duplicar (Cmd+D)`, `Excluir Nota (Del)`, `Selecionar Tudo (Cmd+A)`, `Desmarcar Tudo (Cmd+D)`.
  - **Exibir**: `Aumentar Zoom X (Cmd+=)`, `Diminuir Zoom X (Cmd+-)`, `Resetar Zoom (Cmd+0)`, `Alternar Waveform`, `Alternar Gaveta de Parâmetros`, `Janela de Logs de Renderização`.
  - **Inserir**: `Inserir Nova Nota C4`, `Inserir Nota no Cursor`, `Importar MIDI...`, `Importar UST...`, `Importar USTX...`.
  - **Reproduzir**: `Tocar / Pausar (Espaço)`, `Parar e Ir para o Início (Esc)`, `Rebobinar (0ms)`, `Ir para o Final`, `Pré-renderizar Áudio`.
  - **Ferramentas**: `Ponteiro (V)`, `Lápis (N)`, `Pincel de Pitch (P)`, `Borracha (E)`, `Copaiba Voicebank Toolkit`.
  - **Janela**: `Galeria de Cantores`, `Gerenciar Pastas de Cantores`, `Painel Lateral Direito`, `Gaveta Inferior de Parâmetros`.
  - **Ajuda**: `Guia de Teclas de Atalho (F1 / Cmd+?)`, `Discord Rich Presence`, Informações de Versão.
- **Painel Lateral Direito Unificado**:
  - Aba **Cantor & Faixa**: Avatar em alta resolução, autor, informações do `readme`, botão de galeria e lista de cantores detectados.
  - Aba **Nota**: Edição de tom, duração, letra, parâmetros fonéticos e envelopes de volume.
  - Aba **Fonemas**: Paleta de fonemas disponíveis no voicebank com inserção rápida por duplo clique.
  - Aba **Motor de Síntese**: Seleção de resampler (`TD-PSOLA`, `SOLA`, `macres`, etc.), wavtool e taxa de amostragem (`44100 Hz`, `48000 Hz`, `96000 Hz`).

---

### Copaiba Voicebank Toolkit
- **Novo Formato `copaiba.config` (JSON)**: Estrutura JSON moderna para configuração de voicebanks com suporte a corte inicial, consoante, loop, cauda final e corte final.
- **Visualizador Triplo de Formas de Onda (Stacked 3-Waveform View)**:
  - Forma de onda do alias anterior para alinhamento contínuo.
  - Forma de onda ativa (central) com marcadores interativos de clique e arraste.
  - Forma de onda do próximo alias.
- **Executável Independente e Janela Integrada**: Disponível via binário dedicado (`cargo run --bin copaiba`) e acessível diretamente no menu de ferramentas do Kamafeu Studio.

---

### Pipeline de Releases Multiplataforma & Suporte a Windows 32-bit
- **Suporte a Windows x86 (32-bit)**: Adicionado o target `i686-pc-windows-msvc` no pipeline automatizado de CI/CD do GitHub Actions.
- **Empacotamento Automatizado**: Geração de pacotes `.zip` contendo os binários `kamafeu.exe` e `copaiba.exe` para arquiteturas Windows de 32 e 64 bits.
- **Notificações Integradas**: Publicação e notificação com links de download direto para Windows x64, Windows x86 (32 bits), macOS (Apple Silicon e Intel) e Linux x64.

---

### Cobertura de Testes Automatizados
- **59/59 testes unitários** passando com 100% de aprovação, cobrindo DSP de envelopes, curvas de vibrato, pitch bend solvers, encoders base64 de UTAU, drivers de resamplers, parsers UST/USTX, fonemizadores Romaji/BRAPA/English/Português e renderização multifaixa.

---

## [0.1.0-alpha.1] - 2026-07-30

- Versão Alpha inicial do Kamafeu Synthesizer Core.
- Suporte a leitura de Voicebanks UTAU (`oto.ini`, `prefix.map`).
- Engine gráfica desenvolvida em Rust com `eframe` / `egui`.
