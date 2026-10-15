//! Shared, deliberately small policy for time-based UI work.
//!
//! Keeping the decisions here makes it possible to test animation cadence
//! without constructing an egui context, and prevents individual widgets from
//! accidentally keeping the whole application at 120 Hz.

use std::time::Duration;

pub const NORMAL_FRAME_MS: f32 = 1000.0 / 60.0;
pub const REDUCED_FRAME_MS: f32 = 1000.0 / 30.0;
pub const HIGH_FRAME_TIME_MS: f32 = 24.0;
pub const MIN_ANIMATION_SPEED: f32 = 0.25;
pub const MAX_ANIMATION_SPEED: f32 = 4.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationQuality {
    Normal,
    Reduced,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationPolicy {
    pub enabled: bool,
    pub reduce_motion: bool,
    pub speed: f32,
    pub frame_time_ema_ms: f32,
}

impl AnimationPolicy {
    pub fn normalized_speed(self) -> f32 {
        self.speed.clamp(MIN_ANIMATION_SPEED, MAX_ANIMATION_SPEED)
    }

    pub fn quality(self) -> AnimationQuality {
        if self.reduce_motion || self.frame_time_ema_ms >= HIGH_FRAME_TIME_MS {
            AnimationQuality::Reduced
        } else {
            AnimationQuality::Normal
        }
    }

    pub fn interval(self) -> Option<Duration> {
        self.enabled.then(|| {
            let ms = match self.quality() {
                AnimationQuality::Normal => NORMAL_FRAME_MS,
                AnimationQuality::Reduced => REDUCED_FRAME_MS,
            };
            Duration::from_secs_f32(ms / 1000.0)
        })
    }

    pub fn allows_motion(self) -> bool {
        self.enabled && !self.reduce_motion && self.quality() == AnimationQuality::Normal
    }

    /// Shared visual values keep the piano roll, footer and general chrome in
    /// the same visual family. Reduced quality is deliberately still useful:
    /// it dims continuous effects without removing state changes.
    pub fn pulse_amplitude(self) -> f32 {
        if !self.enabled || self.reduce_motion {
            0.0
        } else if self.quality() == AnimationQuality::Reduced {
            0.45
        } else {
            0.72
        }
    }

    pub fn visual_speed(self) -> f32 {
        self.normalized_speed()
            * if self.quality() == AnimationQuality::Reduced {
                0.72
            } else {
                1.0
            }
    }

    pub fn transition_duration(self, normal_seconds: f32) -> f32 {
        if !self.enabled {
            0.0
        } else {
            let quality_scale = if self.quality() == AnimationQuality::Reduced {
                0.65
            } else {
                1.0
            };
            (normal_seconds * quality_scale / self.normalized_speed()).max(0.0)
        }
    }
}

pub fn normalized_progress(elapsed: f32, duration: f32) -> f32 {
    if duration <= 0.0 {
        1.0
    } else {
        (elapsed / duration).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(ema: f32) -> AnimationPolicy {
        AnimationPolicy {
            enabled: true,
            reduce_motion: false,
            speed: 1.0,
            frame_time_ema_ms: ema,
        }
    }

    #[test]
    fn normal_cadence_targets_sixty_fps() {
        assert_eq!(policy(16.0).quality(), AnimationQuality::Normal);
        assert!(
            (policy(16.0).interval().unwrap().as_secs_f32() * 1000.0 - NORMAL_FRAME_MS).abs()
                < 0.01
        );
    }

    #[test]
    fn high_frame_time_falls_back_to_thirty_fps() {
        assert_eq!(policy(24.0).quality(), AnimationQuality::Reduced);
        let interval_ms = policy(40.0).interval().unwrap().as_secs_f32() * 1000.0;
        assert!((interval_ms - REDUCED_FRAME_MS).abs() < 0.01);
    }

    #[test]
    fn disabled_animation_has_no_repaint_schedule() {
        assert_eq!(
            AnimationPolicy {
                enabled: false,
                ..policy(16.0)
            }
            .interval(),
            None
        );
    }

    #[test]
    fn reduced_motion_removes_motion_but_keeps_state_transitions() {
        let reduced = AnimationPolicy {
            reduce_motion: true,
            ..policy(16.0)
        };
        assert!(!reduced.allows_motion());
        assert!(reduced.interval().is_some());
    }

    #[test]
    fn progress_is_monotonic_and_bounded() {
        let values = [-1.0, 0.0, 0.5, 1.0, 2.0].map(|x| normalized_progress(x, 1.0));
        assert_eq!(values, [0.0, 0.0, 0.5, 1.0, 1.0]);
    }

    #[test]
    fn pulse_amplitude_is_bounded_by_quality_and_preferences() {
        assert!((policy(16.0).pulse_amplitude() - 0.72).abs() < f32::EPSILON);
        assert!((policy(40.0).pulse_amplitude() - 0.45).abs() < f32::EPSILON);
        assert_eq!(
            AnimationPolicy {
                reduce_motion: true,
                ..policy(16.0)
            }
            .pulse_amplitude(),
            0.0
        );
        assert_eq!(
            AnimationPolicy {
                enabled: false,
                ..policy(16.0)
            }
            .pulse_amplitude(),
            0.0
        );
    }

    #[test]
    fn visual_speed_clamps_and_respects_user_speed() {
        assert_eq!(
            AnimationPolicy {
                speed: 2.0,
                ..policy(16.0)
            }
            .visual_speed(),
            2.0
        );
        assert_eq!(
            AnimationPolicy {
                speed: 0.01,
                ..policy(16.0)
            }
            .visual_speed(),
            MIN_ANIMATION_SPEED
        );
        assert!(
            (AnimationPolicy {
                speed: 2.0,
                ..policy(40.0)
            }
            .visual_speed()
                - 1.44)
                .abs()
                < 0.001
        );
    }

    #[test]
    fn transitions_are_disabled_or_scaled_by_quality() {
        assert_eq!(
            AnimationPolicy {
                enabled: false,
                ..policy(16.0)
            }
            .transition_duration(0.2),
            0.0
        );
        assert!((policy(16.0).transition_duration(0.2) - 0.2).abs() < 0.001);
        assert!(
            AnimationPolicy {
                reduce_motion: true,
                ..policy(16.0)
            }
            .transition_duration(0.2)
                < 0.2
        );
    }
}
