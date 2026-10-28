# Tainah / Tainah

## Português (Brasil)

Tainah é o resampler WORLD do Kamafeu. Ele analisa F0, envoltória espectral e aperiodicidade; o mapa temporal usa offset, cutoff e marcas de `oto.ini` para sustain e cauda. Quando o carregamento, a análise ou a saída falha nos critérios técnicos, o render retorna erro com diagnóstico; não troca para SOLA ou resize. Os critérios detectam silêncio e cobertura insuficiente, mas não garantem qualidade perceptiva. Os créditos das referências de timing e a proveniência da base de compilação estão em [NOTICE.md](../../NOTICE.md).

### Instalação e seleção

Instale uma versão desktop compatível do Kamafeu. Na faixa, abra as opções de voz/resampler e selecione **Tainah**. **Straycat** continua como padrão desktop enquanto não houver avaliação auditiva controlada que aprove Tainah. O Kamafeu Web não oferece este resampler nativo.

Para desenvolvimento local, execute `python3 native/tainah/build.py` e inicie o aplicativo; ele procura automaticamente a biblioteca em `target/tainah-package`. `TAINAH_LIBRARY_PATH` continua disponível para indicar outro caminho. O script fixa a revisão do Tainah-rt e sua base lessampler-ng, compila o adaptador em `native/tainah` e verifica ABI, mapa temporal e equivalência entre CLI e biblioteca. A biblioteca usada pelo aplicativo é `tainah_motor.dll`, `libtainah_motor.dylib` ou `libtainah_motor.so.1`. Bibliotecas anteriores sem a extensão de tempos são rejeitadas com diagnóstico. O cache de análise fica na pasta de cache do usuário, fora do voicebank.

O motor usa os detectores WORLD Harvest ou DIO. Quando as preferências globais estiverem em YIN ou pyIN, Tainah usa Harvest; ao escolher DIO, o motor usa DIO.

### Voicebank e oto.ini

Use um voicebank com WAVs acessíveis e entradas válidas em `oto.ini`. Offset, consonant, cutoff, preutterance e overlap são dados de timing importantes; para esticar uma vogal, marcas de loop/sustain e tail permitem preservar o ataque e a cauda. Alias ausente, arquivo WAV ilegível, região útil menor que 64 amostras, análise sem frames ou saída silenciosa pode impedir o render WORLD. Consulte o diagnóstico do projeto para identificar o erro.

### Flags

As flags são sensíveis a maiúsculas/minúsculas. Valores fora do intervalo são limitados. `g` (−100 a +100) altera a inclinação espectral sem alterar F0; `t` (−100 a +100) soma 10 cents por unidade à curva de pitch uma única vez; `B` (0 a 100) interpola a aperiodicidade medida em direção a 0,95; `H` (0 a 99) aplica um corte logarítmico na envoltória, de Nyquist até 1 kHz. Tokens não suportados permanecem no projeto e aparecem nos diagnósticos sem interromper o render. Flags efetivas fazem parte da chave de cache do render.

### Stretch e compatibilidade

O limite original de consoante é uma duração após offset. A velocidade muda a duração dessa região na saída, sem deslocar seu limite na gravação. A região de vogal é alongada quando necessário; notas curtas utilizam apenas a parte necessária, sem acelerar o restante da gravação. A análise WORLD usa o WAV completo para preservar contexto em torno dos cortes de `oto.ini`; o render respeita a região selecionada.

A CLI interpreta cada ponto de pitch como cinco ticks musicais, usando o BPM recebido. Na ABI 2, os campos opcionais `target_consonant_ms` e `has_target_consonant` controlam a duração desejada; estruturas antigas mantêm o tempo original quando não solicitam esse controle. A flag `C` ajusta a duração de saída e `P` controla formantes. Os campos existentes conservam posição e significado. SOLA continua disponível como driver clássico selecionado separadamente. Outros resamplers podem interpretar flags de forma diferente.

### Estado de avaliação

Não há benchmark público concluído nem resultado de escuta cega aprovado. O relatório interno [`engine-audit.md`](kfv2/engine-audit.md) registra o comportamento conhecido e os limites das medições. Não publique renders de voicebanks sem autorização explícita.

## English

Tainah is Kamafeu's WORLD resampler. It analyzes F0, spectral envelope, and aperiodicity. Its time map uses offset, cutoff, and `oto.ini` markers for sustain and tail regions. Library loading, analysis, or output failures return diagnostics; it does not switch to SOLA or resize. These checks detect silence and insufficient coverage; they do not establish perceptual quality. Timing reference credits and build-base provenance are listed in [NOTICE.md](../../NOTICE.md).

### Installation and selection

Install a compatible Kamafeu desktop build. In the track voice/resampler options, select **Tainah**. **Straycat** remains the desktop default until Tainah passes a controlled listening evaluation. The Kamafeu Web build does not provide this native resampler.

For local development, run `python3 native/tainah/build.py` and launch the app; it automatically searches `target/tainah-package`. `TAINAH_LIBRARY_PATH` is available to select another path. The script pins Tainah-rt and its lessampler-ng base, builds the adapter in `native/tainah`, and checks the ABI, time map, and CLI/library equivalence. The app loads `tainah_motor.dll`, `libtainah_motor.dylib`, or `libtainah_motor.so.1`. Older libraries without timing support are rejected with a diagnostic. Analysis cache files live in the user's cache directory, outside the voicebank.

The engine uses WORLD Harvest or DIO. If the global preference is YIN or pyIN, Tainah uses Harvest; selecting DIO makes it use DIO.

### Voicebanks and oto.ini

Use a voicebank with readable WAV files and valid `oto.ini` entries. Offset, consonant, cutoff, preutterance, and overlap affect timing. For long vowels, loop/sustain and tail markers help preserve the attack and ending. Missing aliases, unreadable WAVs, useful regions shorter than 64 samples, failed analysis, or silent output can prevent a WORLD render. Check project diagnostics to see the error.

### Flags

Flags are case sensitive. Out of range values are clamped. `g` (−100 to +100) changes spectral tilt without changing F0; `t` (−100 to +100) adds 10 cents per unit to the pitch curve once; `B` (0 to 100) interpolates measured aperiodicity toward 0.95; `H` (0 to 99) applies a logarithmic envelope cutoff from Nyquist down to 1 kHz. Unsupported tokens remain in project data and appear in diagnostics without stopping the render. Effective flags are included in the render cache key.

### Stretch and compatibility

The source consonant boundary is a duration after offset. Velocity changes its output duration without moving that boundary in the recording. Vowels are stretched when needed; short notes use only the required portion instead of accelerating the remaining recording. WORLD analyzes the complete WAV to retain context around oto.ini cuts; rendering respects the selected region.

The CLI places pitch points five musical ticks apart using the supplied BPM. Optional ABI 2 fields `target_consonant_ms` and `has_target_consonant` specify output timing; older structures retain source timing when they do not request this control. Flag `C` adjusts output duration and `P` controls formants. Existing fields retain their offsets and meanings. SOLA remains available as a separately selected classic driver. Other resamplers may interpret flags differently.

### Evaluation status

There is no completed public benchmark or approved blind listening result. The internal [`engine audit`](kfv2/engine-audit.md) records known behavior and measurement limits. Do not publish voicebank renders without explicit authorization.
