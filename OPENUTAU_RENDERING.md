# Adaptação do pipeline de canto do OpenUtau

Esta implementação foi estudada a partir do [repositório oficial do OpenUtau](https://github.com/openutau/OpenUtau) (licença MIT), especialmente `UPhoneme`, `ResamplerItem`, `SharpWavtool`, `RenderPhrase` e `UVibrato`. O Kamafeu mantém uma implementação Rust própria e um modelo temporal em milissegundos.

## Temporização fonética

- A velocidade consonantal usa `stretch = 2^(1 - VEL/100)`.
- `preutterance` e `overlap` são escalados juntos para conservar a proporção do `oto.ini`.
- Em notas adjacentes, a região sem overlap não pode ocupar mais da metade da nota anterior; plosivas e overlaps negativos recebem limites próprios.
- Cada fonema informa `leading`, `tailIntrude` e `tailOverlap`; esses valores alimentam duração, envelope e posição no wavtool.
- A duração enviada a resamplers clássicos é arredondada em blocos de 50 ms com o bloco de segurança usado pelo OpenUtau.

## Resampler e pitch

O pitch entregue ao motor é amostrado a cada 5 ms e combina `PITD`, pontos Mode 2 e vibrato. A mesma curva é usada pelo TD-PSOLA nativo e codificada em Base64 de 12 bits para resamplers UTAU externos. `VOL`, `MOD`, flags, BPM, recorte e região consonantal seguem no contrato CLI clássico.

Notas adjacentes usam um portamento real de frase. Por padrão, dois pontos cercam o início da nova nota; o primeiro pode ser preso ao tom anterior (`snap_first`) e o segundo alcança o tom atual. Como todos os fonemas amostram essa mesma curva absoluta, gravações sobrepostas nunca recebem afinações conflitantes. O eixo X dos pontos USTX é lido e escrito diretamente em milissegundos.

## Wavtool e envelope

O envelope final possui cinco pontos calculados no espaço do fonema:

- início em `-preutterance`;
- ataque derivado do overlap ou de um mínimo de 5 ms;
- cauda em `duration - tailIntrude + tailOverlap`;
- release derivado da próxima sobreposição ou de um mínimo de 35 ms;
- níveis determinados por `VOL`, `ATK` e `DEC`.

Antes do crossfade complementar, o wavtool interno procura um pequeno deslocamento por correlação normalizada. Essa compensação de fase é a adaptação Rust da finalidade do modo de convergência do `SharpWavtool`.

## Vibrato e dinâmica

O vibrato suporta comprimento, período, profundidade, fades percentuais, fase, drift e vínculo de volume. `DYN` é armazenado em unidades de 0,1 dB e convertido para ganho linear na renderização. O vínculo de volume do vibrato é aplicado por amostra antes da mixagem fonética.

## Compatibilidade

- Projetos antigos recebem defaults por `serde` e continuam carregando.
- UST lê e escreve `VBR`, `Envelope`, `Intensity`, `Modulation` e `Flags`.
- USTX lê e escreve `vibrato` e expressões `vel`, `dyn`, `vol`, `atk`, `dec`, `mod`, `bre`, `gen` e `pitd`.
