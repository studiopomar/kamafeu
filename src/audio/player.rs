use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    active_sink: Option<Sink>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        eprintln!("[AudioPlayer] Initializing CoreAudio OutputStream on main thread...");
        let (stream, stream_handle) = match OutputStream::try_default() {
            Ok((s, h)) => {
                eprintln!("[AudioPlayer] CoreAudio device initialized successfully!");
                (Some(s), Some(h))
            }
            Err(e) => {
                eprintln!("[AudioPlayer] ERROR initializing CoreAudio device: {}", e);
                (None, None)
            }
        };

        Self {
            _stream: stream,
            stream_handle,
            active_sink: None,
        }
    }

    pub fn play_samples(&mut self, samples: Vec<f32>, sample_rate: u32) {
        self.play_samples_with_channels(samples, sample_rate, 1);
    }

    pub fn play_samples_with_channels(
        &mut self,
        samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) {
        if samples.is_empty() {
            return;
        }

        if let Some(old_sink) = self.active_sink.take() {
            old_sink.pause();
            old_sink.detach();
        }

        if let Some(ref handle) = self.stream_handle {
            match Sink::try_new(handle) {
                Ok(sink) => {
                    let buffer = SamplesBuffer::new(channels.max(1), sample_rate, samples);
                    sink.append(buffer);
                    sink.play();
                    self.active_sink = Some(sink);
                }
                Err(err) => eprintln!("[AudioPlayer] Error creating Sink: {}", err),
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.active_sink.take() {
            sink.pause();
            sink.detach();
        }
    }

    pub fn is_playing(&self) -> bool {
        if let Some(ref sink) = self.active_sink {
            return !sink.empty();
        }
        false
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_player_play_samples() {
        let mut player = AudioPlayer::new();
        let samples = vec![0.1f32; 4410];
        player.play_samples(samples, 44100);
        assert!(player.is_playing() || player.stream_handle.is_none());
        player.stop();
        assert!(!player.is_playing());
    }
}
