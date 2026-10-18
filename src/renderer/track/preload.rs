use super::{io_thread_count, TrackRenderer, LAZY_WAV_LOADING};
use crate::oto::Voicebank;
use crate::phonemizer::RenderPhone;
use crate::project::model::UNote;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub(super) fn prepare_voicebank_audio(
    notes: &[UNote],
    phones: &mut Vec<RenderPhone>,
    voicebank: &Voicebank,
    log: &impl Fn(f32, &str),
) -> HashMap<PathBuf, Arc<(Vec<f32>, u32)>> {
    // ------------------------------------------------------------------
    // Phase 0: Pre-load all unique WAV files into a shared in-memory
    // cache.  This eliminates repeated disk reads for voicebanks where
    // many notes share the same sample file (e.g. all 'あ' notes use the
    // same あ.wav).  The load itself is also parallelised with rayon.
    // ------------------------------------------------------------------
    let unique_wav_paths: std::collections::HashSet<std::path::PathBuf> = phones
        .iter()
        .filter_map(|phone| {
            phone
                .find_entry(voicebank)
                .map(|entry| voicebank.root_path.join(&entry.wav_filename))
        })
        .collect();

    // Collect paths into a Vec so rayon can index them.
    let wav_paths_vec: Vec<std::path::PathBuf> = if LAZY_WAV_LOADING.load(Ordering::Relaxed) {
        Vec::new()
    } else {
        unique_wav_paths.into_iter().collect()
    };
    #[cfg(not(target_arch = "wasm32"))]
    let io_threads = io_thread_count().read().map(|value| *value).unwrap_or(4);
    #[cfg(target_arch = "wasm32")]
    let io_threads = 1;
    #[cfg(not(target_arch = "wasm32"))]
    let wav_cache: HashMap<std::path::PathBuf, Arc<(Vec<f32>, u32)>> = if wav_paths_vec.is_empty() {
        HashMap::new()
    } else {
        let load_wavs = || {
            wav_paths_vec
                .into_par_iter()
                .filter_map(|path| {
                    TrackRenderer::load_wav_samples(&path)
                        .ok()
                        .map(|(samples, rate)| (path, Arc::new((samples, rate))))
                })
                .collect()
        };
        match crate::renderer::worker_pool::get(io_threads) {
            Ok(pool) => pool.install(load_wavs),
            Err(_) => load_wavs(),
        }
    };
    #[cfg(target_arch = "wasm32")]
    let wav_cache: HashMap<std::path::PathBuf, Arc<(Vec<f32>, u32)>> = wav_paths_vec
        .into_iter()
        .filter_map(|path| {
            TrackRenderer::load_wav_samples(&path)
                .ok()
                .map(|(samples, rate)| (path, Arc::new((samples, rate))))
        })
        .collect();
    // Never substitute a missing phoneme with generated audio. Keep its
    // time slot silent and resolve transitions only between available phones.
    let unavailable = phones
        .iter()
        .map(|phone| {
            phone.find_entry(voicebank).is_none_or(|entry| {
                let path = voicebank.root_path.join(&entry.wav_filename);
                if LAZY_WAV_LOADING.load(Ordering::Relaxed) {
                    !path.is_file()
                } else {
                    wav_cache.get(&path).is_none_or(|audio| audio.0.is_empty())
                }
            })
        })
        .collect::<Vec<_>>();
    let missing_inside_phrase = unavailable.iter().enumerate().any(|(index, missing)| {
        if !missing {
            return false;
        }
        let phone = &phones[index];
        let touches_previous = index > 0
            && (phones[index - 1].position_ms + phones[index - 1].duration_ms - phone.position_ms)
                .abs()
                <= 0.001;
        let touches_next = phones.get(index + 1).is_some_and(|next| {
            (phone.position_ms + phone.duration_ms - next.position_ms).abs() <= 0.001
        });
        touches_previous || touches_next
    });
    phones.retain(|phone| {
        let reason = match phone.find_entry(voicebank) {
            None => Some("alias ausente no voicebank".to_string()),
            Some(entry) => {
                let path = voicebank.root_path.join(&entry.wav_filename);
                let available = if LAZY_WAV_LOADING.load(Ordering::Relaxed) {
                    path.is_file()
                } else {
                    wav_cache
                        .get(&path)
                        .is_some_and(|audio| !audio.0.is_empty())
                };
                (!available)
                    .then(|| format!("amostra WAV ausente, vazia ou ilegível: {}", path.display()))
            }
        };
        if let Some(reason) = reason {
            log(
                0.0,
                &format!(
                    "[Warning] Fonema '{}' ({}) em {:.0}ms silenciado: {reason}",
                    phone.lyric, phone.pitch, phone.position_ms
                ),
            );
            false
        } else {
            true
        }
    });
    if missing_inside_phrase {
        // The phonemizer has already shortened the preceding vowel to make
        // room for the missing bridge. Restore continuity up to the next
        // available phone instead of leaving a hole or aborting unrelated
        // phrases. The warning above still exposes the voicebank defect.
        for index in 0..phones.len().saturating_sub(1) {
            let current_note = phones[index].note_index;
            let next_note = phones[index + 1].note_index;
            let same_phrase = current_note == next_note
                || notes
                    .get(current_note)
                    .zip(notes.get(next_note))
                    .is_some_and(|(current, next)| {
                        (current.position_ms + current.duration_ms - next.position_ms).abs()
                            <= 0.001
                    });
            let next_position = phones[index + 1].position_ms;
            let current_end = phones[index].position_ms + phones[index].duration_ms;
            if same_phrase && next_position > current_end + 0.001 {
                phones[index].duration_ms = next_position - phones[index].position_ms;
            }
        }
        log(
        0.0,
        "[Warning] A geometria da frase foi recomposta ao redor de aliases ausentes; a vogal anterior foi preservada até o próximo fonema disponível.",
    );
    }
    wav_cache
}
