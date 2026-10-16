# Tainah / Tainah

## Português (Brasil)

Tainah é o resampler nativo do Kamafeu baseado em WORLD. Ele analisa F0, envoltória espectral e aperiodicidade; o mapa temporal usa offset, cutoff e marcas OTO de sustain/cauda. Quando a análise ou a saída falha nos critérios técnicos, o render pode recorrer a SOLA ou resize preservando pitch. O caminho efetivo e a razão do fallback aparecem nos diagnósticos por fonema. Esses critérios detectam silêncio e cobertura insuficiente, não garantem qualidade perceptiva.

### Instalação e seleção

Instale uma versão desktop compatível do Kamafeu. Na faixa, abra as opções de voz/resampler e selecione **Tainah**. **Straycat** continua como padrão desktop enquanto não houver avaliação auditiva controlada que aprove Tainah. O Kamafeu Web não oferece este resampler nativo.

### Voicebank e OTO

Use um voicebank com WAVs acessíveis e entradas válidas em `oto.ini`. Offset, consonant, cutoff, preutterance e overlap são dados de timing importantes; para esticar uma vogal, marcas de loop/sustain e tail permitem preservar o ataque e a cauda. Alias ausente, arquivo WAV ilegível, região útil menor que 64 amostras, análise sem frames ou saída silenciosa pode resultar em fallback ou silêncio. Consulte o diagnóstico do projeto para identificar a rota efetiva.

### Flags

As flags são sensíveis a maiúsculas/minúsculas. Valores fora do intervalo são limitados. `g` (−100 a +100) desloca a envoltória espectral em até meia oitava sem alterar F0; `t` (−100 a +100) soma 10 cents por unidade à curva de pitch uma única vez; `B` (0 a 100) interpola a aperiodicidade medida em direção a 0,95; `H` (0 a 99) aplica um corte logarítmico na envoltória, de Nyquist até 1 kHz. Tokens não suportados permanecem no projeto e aparecem nos diagnósticos sem interromper o render. Flags efetivas fazem parte da chave de cache do render.

### Stretch e compatibilidade

Em notas longas, Tainah mapeia os frames WORLD sobre a duração solicitada e tenta preservar ataque e cauda OTO. O fallback SOLA permanece disponível no caminho clássico. Outros resamplers podem interpretar flags de forma diferente.

### Estado de avaliação

Não há benchmark público concluído nem resultado de escuta cega aprovado. O relatório interno [`engine-audit.md`](kfv2/engine-audit.md) registra o comportamento conhecido e os limites das medições. Não publique renders de voicebanks sem autorização explícita.

## English

Tainah is Kamafeu's native WORLD based resampler. It analyzes F0, spectral envelope, and aperiodicity. Its time map uses offset, cutoff, and OTO sustain/tail markers. When analysis or output fails technical coverage checks, rendering may fall back to SOLA or pitch preserving resize. Per phoneme diagnostics record the effective path and fallback reason. These checks detect silence and insufficient coverage; they do not establish perceptual quality.

### Installation and selection

Install a compatible Kamafeu desktop build. In the track voice/resampler options, select **Tainah**. **Straycat** remains the desktop default until Tainah passes a controlled listening evaluation. The Kamafeu Web build does not provide this native resampler.

### Voicebanks and OTO

Use a voicebank with readable WAV files and valid `oto.ini` entries. Offset, consonant, cutoff, preutterance, and overlap affect timing. For long vowels, loop/sustain and tail markers help preserve the attack and ending. Missing aliases, unreadable WAVs, useful regions shorter than 64 samples, failed analysis, or silent output can cause fallback or silence. Check project diagnostics to see the effective route.

### Flags

Flags are case sensitive. Out of range values are clamped. `g` (−100 to +100) shifts the spectral envelope by up to half an octave without changing F0; `t` (−100 to +100) adds 10 cents per unit to the pitch curve once; `B` (0 to 100) interpolates measured aperiodicity toward 0.95; `H` (0 to 99) applies a logarithmic envelope cutoff from Nyquist down to 1 kHz. Unsupported tokens remain in project data and appear in diagnostics without stopping the render. Effective flags are included in the render cache key.

### Stretch and compatibility

For long notes, Tainah maps WORLD frames to the requested duration and attempts to preserve OTO attack and tail regions. SOLA fallback remains available in the classic rendering path. Other resamplers may interpret flags differently.

### Evaluation status

There is no completed public benchmark or approved blind listening result. The internal [`engine audit`](kfv2/engine-audit.md) records known behavior and measurement limits. Do not publish voicebank renders without explicit authorization.
