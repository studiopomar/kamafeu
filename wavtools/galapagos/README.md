# Galapagos Wavtool

Galapagos helps a synthesizer shape and join rendered audio segments. It is a
small Rust library with no third-party dependencies, so you can add it to a
Rust project or adapt the source for another host under the MIT license.

It applies a five-point, piecewise-linear envelope and can adjust a segment's
position inside an existing overlap. When a fundamental frequency is
available, the search checks a small range of sample positions and chooses
the one with the highest combined energy. This can help avoid phase
cancellation between periodic sounds. The search is limited to half a period,
5 ms, and the actual overlap, whichever is smaller. It does not change the
envelope or normalize the mix.

## Add it to a Rust project

```toml
galapagos-wavtool = "0.1"
```

Then apply your envelope and mix a rendered segment:

```rust
use galapagos_wavtool::{mix_with_optimized_overlap, EnvelopePoint};

let points = [
    EnvelopePoint::new(-40.0, 0.0),
    EnvelopePoint::new(-10.0, 1.0),
    EnvelopePoint::new(0.0, 1.0),
    EnvelopePoint::new(480.0, 1.0),
    EnvelopePoint::new(520.0, 0.0),
];

let mut phrase = vec![0.0; 48_000];
let mut phoneme = vec![0.0; 24_000];
let correction_samples = mix_with_optimized_overlap(
    &mut phrase,
    &mut phoneme,
    12_000, // nominal start
    24_000, // end of the previous segment
    48_000, // sample rate
    -40.0,  // envelope time origin, in milliseconds
    &points,
    Some(220.0), // estimated fundamental frequency, in Hz
);
println!("placement adjustment: {correction_samples} samples");
```

The functions work with mono `f32` samples. For stereo, run the overlap search
on one channel and apply the returned placement adjustment to the other
channel too. See the Rust API for lower-level envelope and overlap functions.

## Check it locally

```sh
cargo test
```

## Galapagos or Andromeda?

Both libraries apply an envelope and mix rendered audio into a phrase. The
difference is how they shape and place each segment:

| Library | Envelope | Segment placement |
| --- | --- | --- |
| **Galapagos** | Five-point linear envelope | Can search a small, bounded range of sample positions to reduce phase cancellation when the fundamental frequency is known. |
| [**Andromeda**](https://github.com/studiopomar/andromeda) | Five-point cosine envelope | Adds the segment at the position supplied by the host; it does not adjust the timing. |

Choose Galapagos if you want the optional overlap search. Choose Andromeda if
you want cosine shaping and straightforward additive mixing.

## Compatibility

Galapagos is a reusable algorithm library. It is not currently a UTAU command
line wavtool or a drop-in replacement for the append-file protocol. Your host
remains responsible for phonemization, resampling, timing, and audio output.

The overlap search is inspired by public descriptions of wavtool4vcv. This
repository contains no wavtool4vcv code and is not affiliated with UTAU,
OpenUtau, or nmasao. See [NOTICE.md](NOTICE.md) and [LICENSE](LICENSE) for
attribution and reuse terms.
