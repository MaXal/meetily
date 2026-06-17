// diarization/session.rs
//
// Per-recording diarization state: embedding extractor + online clusterer.
// Created when a recording starts (if the feature is enabled and the model
// is present) and dropped when it ends.
//
// This fork integrates diarization with the per-source VAD pipeline: the
// microphone is the local user ("You") and is never diarized, so this session
// only ever sees system-audio speech segments. Each VAD segment is already a
// clean, single-source utterance, so we can embed and cluster it directly via
// `label_segment` — no rolling window / overlap timeline is needed here.

use super::clustering::SpeakerClusterer;
use super::embedding::{EmbeddingError, EmbeddingExtractor};
use std::path::Path;

/// Minimum samples needed for the fbank frontend to produce the 10 frames
/// required by EmbeddingExtractor::compute (25ms frame + 9 * 10ms shifts).
const MIN_SAMPLES_FOR_EMBEDDING: usize = 1_840;

/// Anonymous-speaker cap for the system stream. The mic ("You") is handled
/// separately, so this bounds how many distinct remote participants we will
/// mint. Higher than the post-mix default because here every speaker in the
/// cap is a remote participant, not the local user.
const MAX_SYSTEM_SPEAKERS: usize = 6;

fn has_enough_samples_for_embedding(samples_len: usize) -> bool {
    samples_len >= MIN_SAMPLES_FOR_EMBEDDING
}

pub struct DiarizationSession {
    extractor: EmbeddingExtractor,
    clusterer: SpeakerClusterer,
}

impl DiarizationSession {
    pub fn new(embedding_model_path: &Path) -> Result<Self, EmbeddingError> {
        Ok(Self {
            extractor: EmbeddingExtractor::new(embedding_model_path)?,
            clusterer: SpeakerClusterer::with_max_anonymous_speakers(MAX_SYSTEM_SPEAKERS),
        })
    }

    /// Assign a speaker label to a 16kHz mono speech segment.
    /// Returns None only when no label can be produced (e.g. the first segment
    /// is too short). Diarization failures must never break transcription —
    /// errors are logged and degrade to the previous label or None.
    pub fn label_segment(&mut self, samples_16k: &[f32]) -> Option<String> {
        if !has_enough_samples_for_embedding(samples_16k.len()) {
            return self.clusterer.last_label();
        }
        match self.extractor.compute(samples_16k) {
            Ok(embedding) => Some(self.clusterer.assign(&embedding)),
            Err(e) => {
                log::warn!(
                    "Diarization embedding failed, carrying previous label: {}",
                    e
                );
                self.clusterer.last_label()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedding_gate_matches_minimum_fbank_frames() {
        assert!(!has_enough_samples_for_embedding(MIN_SAMPLES_FOR_EMBEDDING - 1));
        assert!(has_enough_samples_for_embedding(MIN_SAMPLES_FOR_EMBEDDING));
    }
}
