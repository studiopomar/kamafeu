# KFV 2 data format

KFV 2 is Kamafeu's versioned, data-only concatenative voicebank format. A
`.kfv` file is a ZIP package whose root contains `kamafeu_voicebank.json`.
Authoring folders use the same layout. This specification describes the
currently implemented contract; it does not certify acoustic suitability or
public-release readiness.

## Normative files

| File | Schema | Purpose |
| --- | --- | --- |
| `kamafeu_voicebank.json` | [manifest schema](schemas/manifest.schema.json) | Stable voice/bank identity, engine requirement, profile/style/layer references and defaults. |
| Referenced phonetic profile | [phonetics schema](schemas/phonetics.schema.json) | Phone inventory and optional direct-pronunciation lexicon. |
| Each layer's units file | [units schema](schemas/units.schema.json) | Audio asset metadata and units with explicit source regions and timing. |

The [minimal example](examples/minimal/) shows all three JSON documents. It is
a structural fixture: `audio/natural/c4/take-1.wav` is intentionally absent,
so it cannot be installed or rendered as a bank.

In Copaíba, importing a source WAV copies its bytes into the authoring tree and
records the copy as an asset. The A/B take comparison reads two assets from the
selected layer, rechecks their declared WAV metadata, and never writes into the
source recordings. Interactive waveform comparison currently limits each
decoded asset to 8,388,608 mono frames.

The library also exposes an assisted OTO-to-KFV2 migration API. It only writes
a new directory outside the source bank, stages and validates that directory,
then renames it into place. Existing destinations are rejected. Per-alias
reports distinguish explicit phone mappings from exact whitespace-token
inferences; root and subbank OTO lines are reread so duplicate aliases are
retained as separate candidates and flagged as collisions. Ambiguous aliases
and unsafe, missing, oversized, stereo, or invalid WAV regions are skipped
with reasons. OTO offset, consonant, cutoff,
loop, and tail marks are converted where their frame ranges are valid.
Millisecond-to-frame conversion uses the classic renderer's truncation rule;
negative offsets clamp to frame zero, positive cutoffs trim from file end,
negative cutoffs count forward from the clamped offset, and zero cutoff uses
the file end. Negative overlap is reported and clamped to zero because KFV 2
crossfade durations cannot represent that legacy value.
The authoring directory retains the complete per-entry report as
`migration_report.json`; it is not included in the manifest content hash or the
public package allowlist.
The optional mapping file accepted by the Copaíba wizard and
`kamafeu migrate-voicebank` CLI is a UTF-8 JSON object from exact OTO alias
strings to one- or two-phone arrays, for example `{"ka": ["k", "a"]}`.
Phone IDs must exist in the chosen profile. Without that file, only exact
whitespace-token inference is attempted. The map is retained in the private
authoring directory as `migration_phone_mappings.json` for review; it is not
included in the public package allowlist.
Preutterance, pitch-dependent prefix maps, voice colors, and context are not
fully represented; the report records these losses. All imported units remain
unreviewed, contexts are generalized, and no license or distribution rights
are inferred. The generated bank is a private draft for Copaíba review, not a
publicly releasable migration. Copaíba exposes this through a guided action:
the creator selects a profile JSON and an output parent folder. The current UI
uses exact alias-token inference only; callers of the library API may also
provide explicit mappings. The UI and CLI can also load explicit mappings from
the optional JSON file.

## Validation layers

JSON Schema checks document shape and primitive constraints. The Kamafeu loader
also enforces rules that span documents or require filesystem inspection:

- KFV manifest `schema_version` must be numeric SemVer 2.x; profile and unit
  documents currently accept 1.x.
- IDs are unique in their declared scope; references resolve to declared
  profiles, styles, layers, assets, and phones.
- Language tags are BCP 47-shaped and profile languages are declared by the
  bank. Paths are relative, contain only normal path components, and resolve
  within the package root, including symlink checks.
- Unit frame coordinates are half-open source-frame positions. The asset must
  be mono PCM WAV and its sample rate and frame count must match the document.
  `start_frame < end_frame`; the anchor, fixed region, sustain, and optional
  release must be ordered and contained in the source interval.
- A looping or stretched sustain requires an explicit sustain region. Missing
  mandatory units are errors; native rendering never substitutes OTO audio.
- `quality.reviewed` records an explicit author review, not an automatic audio
  quality judgment. The planner may use it as a deterministic tie-breaker.
- ZIP packages have entry-count and expanded-size limits and reject duplicate
  paths, traversal, symlinks, and declared/actual size mismatches.

Additional object properties are currently tolerated by Serde. Producers
should emit only fields defined by the schemas; consumers must ignore fields
they do not understand and must reject unsupported schema or engine majors.
KFV packages contain data only and cannot provide executable code.

## Compatibility status

The application currently implements `kamafeu.voicebank` 2.x,
`kamafeu.phonetics` 1.x, and `kamafeu.units` 1.x with engine
`kamafeu.concat` API major 1. These schema files describe that implementation
snapshot. A schema passing structural validation alone does not mean the bank
can be loaded: cross-reference checks, WAV inspection, and safe package
extraction still apply.

The KFV 2 preview renderer currently uses explicit experimental SOLA. The
reference backend remains undecided until an authorized-recording comparison
is completed. This format specification does not claim beta or sound-quality
compatibility.
