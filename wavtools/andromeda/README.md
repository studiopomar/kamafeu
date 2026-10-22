# Andromeda Wavtool

Repository: [https://github.com/studiopomar/andromeda](https://github.com/studiopomar/andromeda)

Andromeda gives a synthesizer two basic tools for joining rendered audio: a
five-point cosine envelope and an additive segment mixer. It is a small Rust
library with no third-party dependencies. Use it as a Cargo dependency or
adapt the source for another host under the MIT license.

Andromeda keeps the work of phonemization, resampling, timing, and audio output
with the host synthesizer. It adds samples directly to the phrase buffer; it
does not normalize the mix, apply a limiter, or move note timing.

## Add it to a Rust project

```toml
andromeda-wavtool = "0.1"
```

Then describe the envelope and mix a rendered segment:

```rust
use andromeda_wavtool::{mix_segment, EnvelopePoint};

let points = [
    EnvelopePoint::new(-40.0, 0.0),
    EnvelopePoint::new(-10.0, 1.0),
    EnvelopePoint::new(0.0, 1.0),
    EnvelopePoint::new(480.0, 1.0),
    EnvelopePoint::new(520.0, 0.0),
];

let mut phrase = vec![0.0; 48_000];
let mut phoneme = vec![0.0; 24_000];
mix_segment(&mut phrase, &mut phoneme, 12_000, 48_000, -40.0, &points);
```

The functions work with mono `f32` samples. For stereo, call `mix_segment`
for each channel with the same timing and envelope.

## Check it locally

```sh
cargo test
```

## Andromeda or Galapagos?

Both libraries apply an envelope and mix rendered audio into a phrase. The
difference is how they shape and place each segment:

| Library | Envelope | Segment placement |
| --- | --- | --- |
| **Andromeda** | Five-point cosine envelope | Adds the segment at the position supplied by the host; it does not adjust the timing. |
| [**Galapagos**](https://github.com/studiopomar/galapagos) | Five-point linear envelope | Can search a small, bounded range of sample positions to reduce phase cancellation when the fundamental frequency is known. |

Choose Andromeda for cosine shaping and straightforward additive mixing.
Choose Galapagos if you want its optional overlap search.

## Compatibility

Andromeda is a reusable algorithm library, not currently a UTAU command line
wavtool or a drop-in replacement for the append-file protocol. See
[NOTICE.md](NOTICE.md) and [LICENSE](LICENSE) for attribution and reuse terms.
