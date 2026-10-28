# GUI framework and visual system

Status: decision for the current desktop refinement cycle

## Framework decision

Keep `eframe`/`egui` as the editor framework. The project already has a broad,
custom Rust editor: the piano roll, phoneme ruler, parameter drawer, transport,
inspector, arrangement view, dialogs, and touch-first layouts all use egui.
Replacing the toolkit would require rebuilding those interactions and retesting
project editing, audio scheduling, native window behavior, Android, and WASM.
No current editor requirement has been identified that egui cannot meet.

| Option | Fit for Kamafeu Studio | Cost and limits |
| --- | --- | --- |
| egui/eframe | Immediate-mode Rust UI with direct custom painting; already used across the editor and current targets. | Retain and improve the existing toolkit. Bespoke dock behavior needs project-owned layout code or an [egui docking crate](https://docs.rs/egui_dock/latest/egui_dock/). |
| Iced | Cross-platform Rust GUI, with a widget model and an interactive 2D [canvas](https://docs.rs/iced/latest/iced/widget/canvas/index.html). | A migration would replace most GUI code and interaction state. It offers no demonstrated piano-roll capability that offsets that cost today. |
| Slint | Declarative UI for desktop, mobile, web, and embedded targets ([platforms](https://slint.dev/faqs)). | The specialized editor would still need custom components and integration. Moving the existing Rust UI would be a substantial rewrite. |
| FTXUI | C++ components for [terminal applications](https://github.com/ArthurSonzogni/FTXUI). | Suitable for a separate console TUI; it is not a graphical editor toolkit and does not replace the Studio UI. |

The current modular workspace uses project-owned movable windows and snap
behavior. If users need tabbed, dockable panels, evaluate `egui_dock` as a
focused addition before considering a framework migration.

## Visual direction

The visual system is independent of the Calafate 3.0.0+ codename. The existing
“Calafate 3.0” preset name remains intact, as do all preset palettes and saved
theme settings. Shared composition and audio-inspired artwork should work
across those palettes. Keep semantic success, warning, error, and meter colors
distinct from decorative accents. The piano roll remains the highest priority
surface; controls should support it through a clear text hierarchy, consistent
spacing, legible focus/hover/active states, and compact but usable targets.

This cycle slightly increases desktop body, button, small-label, and heading
sizes and gives controls more breathing room in the shared egui style. Touch
layouts retain their larger spacing and targets. Existing reduced-motion
settings continue to control animation cadence.

Sliders use a hardware and audio-plugin fader visual language: the rail fills
from its origin to the cursor, and the cursor is rectangular. The fill uses
egui's theme selection color; the cursor uses the existing inactive, hovered,
and active widget colors. This is a global egui visual setting. It does not
change slider orientation, ranges, units, numeric editing, keyboard or pointer
interaction, or controls that hide their value. Individual sliders may still
override egui's global handle or fill options. The visual reference is broad
and does not reproduce a specific WesAudio product.

## Performance audit boundary

The application already tracks an exponentially smoothed frame time and uses
it to reduce animation quality when frames are slow. That is a responsiveness
signal, not a profile of where CPU time or allocations go. Before optimizing,
collect repeatable traces for a small project and a long project with editing,
playback, and rendering measured separately. Record frame-time percentiles and
identify the responsible UI, audio, or render work before changing data flow.
Do not use a successful compile or synthetic benchmark as evidence of desktop
interaction quality.

## Acceptance checks

- Open and create a project; edit notes and phonemes in the piano roll.
- Open and resize the inspector, arrangement, parameter drawer, and dialogs.
- Check theme switching, UI scaling, keyboard focus, and reduced motion.
- Check representative horizontal and vertical faders in light and dark
  themes at multiple UI scales, including drag, click-to-set, numeric editing,
  keyboard adjustment, and sliders with hidden values. Confirm that the fill
  and rectangular cursor remain legible across inactive, hovered, and active
  states.
- Repeat interaction with a long project during playback and rendering, while
  recording frame-time behavior separately from audio/render completion.
- Review native desktop behavior on macOS, Windows, and Linux; keep Android and
  WASM builds compatible with shared UI changes.

The source-level style changes in this cycle are not a substitute for those
interactive checks. No cross-platform visual or long-project performance
acceptance is claimed until those runs are performed.
