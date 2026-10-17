# KFV 2 contract (implementation status: structural preview)

KFV 2 is a versioned JSON voicebank format carried in the existing `.kfv`
ZIP container. The stable discriminator is `format: "kamafeu.voicebank"` and
the current manifest contract is `schema_version: "2.x"`. The manifest is
the authority for this format; `oto.ini` is not a KFV 2 fallback source.

The current implementation introduces Rust models and structural validation
for the manifest, phonetic profiles, assets, and unit regions. The
`VoicebankHandle` facade can validate/open a KFV 2 directory or `.kfv` package;
the classic `Voicebank::new` entrypoint deliberately rejects KFV 2 because it
still returns the legacy model for classic `oto.ini` voicebanks. Existing
`oto.ini` and Copaíba JSON banks continue through their current path.

The native planner selects a deterministic, context-eligible unit, honors
explicit unit/layer overrides, records the decision and anchor timing, and
returns a serializable `PhrasePlan`. `render_phrase_sola` renders that plan to
PCM using an explicitly requested SOLA prototype, without WORLD fallback. The
prototype is not selected as the product reference engine: the authorized
recording A/B spike has not been completed. There is no public beta or product
publishing flow. Copaíba has an authoring preview: it copies mono WAV imports,
creates units, edits source marks with coalesced undo/redo, previews a unit
through the experimental SOLA path, and writes a verified draft package from
manifest-reachable files. Draft packaging is not license or release
certification. Copaíba can also open an existing `.kfv` as an immutable source
snapshot in a private editing workspace and publish edits as a separate draft.
Unit source marks and the author's explicit reviewed flag share coalesced,
ordered undo/redo history; the reviewed flag is persisted in the layer JSON.
Its coverage panel reports symbol presence, unit/asset counts, and reviewed
units. A direct-phone phrase probe also runs the phrase planner against the
selected style and layer, lists each unit decision, reports the first uncovered
event, and can audition the phrase through experimental SOLA. This is
structural planner coverage, not acoustic suitability. Unit and phrase
previews use the current in-memory marking edits, including edits not yet saved
to the authoring workspace. The authoring panel also compares two imported
mono WAV takes from the selected layer side-by-side, shows each source duration,
and plays A or B directly. Comparison rechecks asset metadata and caps decoded
assets at 8,388,608 frames each; it does not edit either source recording.

Copaíba can start a blank PT-BR KFV 2 authoring draft from a small wizard, so
creating a bank no longer requires hand-authoring manifest, profile, and unit
JSON. The draft has one Natural style and MIDI 60 layer, with a starter phone
inventory, but no recordings, G2P/lexicon, or license metadata. It is an
editable local workspace, not a distributable voicebank; the author must add
authorized recordings, review units, and define license/credits before release.

The project model now persists a per-track native bank reference in APS
(stable bank ID/version, package revision, SHA-256 content hash, engine API,
planner/profile/style IDs, and an optional path hint). Per-phoneme unit and
layer override IDs also roundtrip in APS. The singer gallery can bind an
installed native bank to the active track, and the normal project renderer now
routes audible native tracks through their binding while classic tracks keep
the classic voicebank renderer. Native tracks use direct phone sequences and the experimental
SOLA prototype before project gain, pan, effects, wave parts, and range mixing.
Playback and export call that project renderer. This remains a preview path,
not a validated product backend; one- or two-phone notes map to a single KFV 2
event. Longer direct phone sequences are decomposed into overlapping adjacent
transitions, using the note's resolved phoneme
durations; indexed KFV 2 unit/layer overrides apply to those events. It hashes
the bank content on each render to catch changed assets.
The gallery reports structural phone/unit/asset/review counts at binding time,
and the render log records selected unit, layer, pitch, and selection reason
for up to six events per track. These diagnostics do not estimate acoustic
suitability or full phrase coverage.

The package module can inspect a root manifest without inflating WAV entries
and can install a package by validating it in a sibling staging directory and
renaming that directory into place. The singer gallery now lists valid KFV 2
manifest metadata and can install `.kfv` packages into a chosen parent folder.
KFV 2 entries use a native gallery action and are not passed to the legacy
oto.ini voicebank loader. The active track stores stable bank identity plus an optional local
path hint; rendering reopens and verifies that binding.

Native project phrase renders use a bounded in-memory cache keyed by the
verified bank content hash, serialized `PhrasePlan`, track index, output sample
rate, and cache algorithm revision. Content edits, changed selections/overrides,
timing changes, and sample-rate changes therefore miss the cache. Entries are
evicted by oldest insertion when the 64-entry or 64-million-sample budget is
reached. Cancelled renders are never inserted. The cache is process-local and
does not claim persistent-cache behavior across application restarts.

## Contract conventions

- JSON is UTF-8. `voice_id`, `bank_id`, unit IDs, asset IDs, profile IDs, style
  IDs, and layer IDs are stable identifiers and unique in their scope.
- Language tags use BCP 47. MIDI note numbers are absolute, with MIDI 60 as C4.
- All frame coordinates are integer frame positions at the source WAV rate;
  intervals are half-open `[start, end)`. Assets are mono PCM sources.
- Relative paths must consist only of normal path components. Absolute paths,
  parent traversal, backslashes, and NUL bytes are rejected.
- A unit has explicit phones, source bounds, anchor, protected fixed region,
  optional sustain/release, and finite timing limits. Sustain is never inferred.
- A missing required unit is an error. Unsupported schema or engine API majors
  are rejected; they are not interpreted as classic banks.
- KFV 2 contains data only. It does not load executable code from a package.

The structural preview currently supports `kamafeu.voicebank` 2.x,
`kamafeu.phonetics` 1.x, and `kamafeu.units` 1.x. Compatibility guarantees
must not be inferred from these parsers alone. Audio metadata is checked
against the declared WAV metadata when a bank is opened.

The library provides a copy-only assisted migration from entries in classic
`oto.ini` voicebanks to a new KFV 2 directory. It rereads root and subbank
`oto.ini` lines, retains
duplicate aliases as separate candidates, and flags collisions in its
per-entry inference and loss report. It skips ambiguous mappings or invalid
source regions, validates a sibling staging
directory, and rejects an existing destination or a destination inside the
source bank. The authoring directory retains the report as
`migration_report.json`; it is excluded from bank identity and the package
allowlist. Exact alias-token mappings remain marked inferred. Converted
units are unreviewed and use generalized context; preutterance and
pitch-dependent prefix/voice-color behavior are reported as losses. Copaíba
exposes the flow by asking for a phonetic profile JSON and an output parent
folder; without a map it uses exact alias-token inference. The wizard accepts
an optional UTF-8 JSON object mapping exact aliases to one- or two-phone arrays,
validated against the chosen profile. The migration does not infer licensing,
so its output remains a draft requiring manual review before use or
distribution.

## Engine identities

The built-in resampler drivers have stable IDs `kamafeu.sola` and
`kamafeu.world-rs`; display names are separate. Per-phoneme diagnostics record
the selected driver ID, effective backend stages, fallback reasons, adjustments
and wavtool ID. Cache entries carry the same execution metadata in memory and in
a persistent JSON sidecar; cached WAV data without matching metadata is
discarded. Tainah's WORLD route reports analysis, synthesis, and coverage
failures as errors; it does not switch to SOLA or pitch-preserving resize.
KFV 2's explicit SOLA prototype never falls back to WORLD. A product reference backend remains undecided until the
recording A/B spike is complete; see the [engine audit](docs/kfv2/engine-audit.md).

## Validation

The validator checks schema majors, required references, duplicate IDs,
relative paths, mono asset metadata, phone references, source bounds, anchor,
sustain and release ordering, and finite timing/DSP values. Existing files in
folder banks are canonicalized and rejected if a symlink escapes the root. ZIP extraction
uses entry-count and expanded-size limits and rejects traversal, duplicate
paths, symlinks, and declared/actual size mismatches. Project content identity
uses SHA-256 over normalized manifest/profile/unit JSON and referenced asset
bytes. JSON writes and draft package replacement are atomic per file; saving
all authoring metadata is not a single multi-file transaction. Native render
cancellation is checked between phone units, hash reads, WSOLA frames, YIN
periods, pitch marks, and PSOLA grains; a single OS read or bounded inner DSP
window cannot be interrupted. Folder/package equivalence, package installation, project
identity roundtrip, phrase decisions, authoring marks, allowlist draft
packaging, and the SOLA prototype are covered by synthetic tests; these do not
establish comparative sound quality or beta readiness. The end-to-end fixture
also authors a draft, packages and atomically installs it, binds the installed
bank to an APS project, serializes/reopens that project, and renders with the
persisted unit/layer overrides against the installed bank identity.
