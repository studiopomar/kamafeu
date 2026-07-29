use serde::{Deserialize, Serialize};

/// Represents an entry in an UTAU voicebank `oto.ini` file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OtoEntry {
    /// The filename of the audio sample (e.g. `_ka.wav` or `ka.wav`)
    pub wav_filename: String,
    /// The alias used to trigger this sound (e.g. `ka`, `- ka`, `a ka`, or blank if same as wav filename without extension)
    pub alias: String,
    /// Offset from the start of the audio file (in milliseconds)
    pub offset: f64,
    /// Consonant (fixed) length in milliseconds
    pub consonant: f64,
    /// Cutoff from the end of the file (in milliseconds, positive = from end, negative = absolute length from offset)
    pub cutoff: f64,
    /// Pre-utterance duration in milliseconds
    pub preutterance: f64,
    /// Overlap duration in milliseconds
    pub overlap: f64,
}

impl OtoEntry {
    pub fn new(
        wav_filename: String,
        alias: String,
        offset: f64,
        consonant: f64,
        cutoff: f64,
        preutterance: f64,
        overlap: f64,
    ) -> Self {
        Self {
            wav_filename,
            alias,
            offset,
            consonant,
            cutoff,
            preutterance,
            overlap,
        }
    }
}
