use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not(target_arch = "wasm32"))]
#[path = "player/desktop.rs"]
mod desktop;
#[cfg(not(target_arch = "wasm32"))]
#[path = "player/scheduler.rs"]
mod scheduler;
#[cfg(not(target_arch = "wasm32"))]
pub use desktop::AudioPlayer;
#[cfg(not(target_arch = "wasm32"))]
use scheduler::{push_scheduler_event, SchedulerCommand};
#[cfg(target_arch = "wasm32")]
#[path = "player/web_audio.rs"]
mod web_audio;
#[cfg(target_arch = "wasm32")]
pub use web_audio::AudioPlayer;

#[cfg(not(target_arch = "wasm32"))]
pub use scheduler::{PlaybackSchedulerEvent, PlaybackSchedulerState, ProgressivePlaybackHandle};

static FLUSH_DENORMALS_TO_ZERO: AtomicBool = AtomicBool::new(true);

pub fn set_flush_denormals_to_zero(enabled: bool) {
    FLUSH_DENORMALS_TO_ZERO.store(enabled, Ordering::Relaxed);
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
        player.stop();
        assert!(!player.is_playing());
    }

    #[test]
    fn progressive_scheduler_preserves_finite_amplitude() {
        let mut samples = vec![0.97_f32, -0.99_f32, f32::NAN, f32::INFINITY];
        AudioPlayer::sanitize_scheduler_samples(&mut samples);
        assert_eq!(samples[0], 0.97);
        assert_eq!(samples[1], -0.99);
        assert_eq!(samples[2], 0.0);
        assert_eq!(samples[3], 0.0);
    }

    #[test]
    fn stopping_an_inactive_scheduler_does_not_emit_a_transport_stop() {
        let mut player = AudioPlayer::new();
        let _ = player.drain_scheduler_events();
        player.stop_scheduler();
        std::thread::sleep(std::time::Duration::from_millis(30));
        assert!(player.drain_scheduler_events().is_empty());
    }

    #[test]
    fn direct_sink_reports_an_advancing_playback_position_when_audio_is_available() {
        let mut player = AudioPlayer::new();
        if player.prepare_for_playback().is_err() {
            return;
        }
        let samples = vec![0.0_f32; 44_100];
        player
            .try_play_samples_with_channels(samples, 44_100, 1)
            .expect("the initialized output device should accept a direct buffer");
        std::thread::sleep(std::time::Duration::from_millis(80));
        let position_ms = player
            .playback_position_ms()
            .expect("a direct sink should publish its playback position");
        player.stop();
        assert!(
            position_ms > 0.0,
            "direct sink position remained frozen at {position_ms}ms"
        );
    }
}
