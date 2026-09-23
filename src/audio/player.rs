#[cfg(not(target_arch = "wasm32"))]
use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};

#[cfg(not(target_arch = "wasm32"))]
pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    active_sink: Option<Sink>,
    volume: f32,
    speed: f32,
}

#[cfg(not(target_arch = "wasm32"))]
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
            volume: 1.0,
            speed: 1.0,
        }
    }

    pub fn list_output_devices() -> Vec<String> {
        #[allow(unused_imports)]
        use rodio::cpal::traits::{DeviceTrait, HostTrait};
        let host = rodio::cpal::default_host();
        match host.output_devices() {
            Ok(devices) => devices.filter_map(|d| d.name().ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn default_host_name() -> String {
        #[allow(unused_imports)]
        use rodio::cpal::traits::HostTrait;
        let host = rodio::cpal::default_host();
        format!("{:?}", host.id())
    }

    pub fn play_samples(&mut self, samples: Vec<f32>, sample_rate: u32) {
        self.play_samples_with_channels(samples, sample_rate, 1);
    }

    pub fn play_samples_with_channels(
        &mut self,
        mut samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) {
        if samples.is_empty() {
            return;
        }
        Self::sanitize_samples(&mut samples);

        if let Some(old_sink) = self.active_sink.take() {
            old_sink.pause();
            old_sink.detach();
        }

        if let Some(ref handle) = self.stream_handle {
            match Sink::try_new(handle) {
                Ok(sink) => {
                    let buffer = SamplesBuffer::new(channels.max(1), sample_rate, samples);
                    sink.append(buffer);
                    sink.set_volume(self.volume);
                    sink.set_speed(self.speed);
                    sink.play();
                    self.active_sink = Some(sink);
                }
                Err(err) => eprintln!("[AudioPlayer] Error creating Sink: {}", err),
            }
        }
    }

    /// Queue the next contiguous preview chunk without interrupting playback.
    pub fn append_samples_with_channels(
        &mut self,
        mut samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) {
        if samples.is_empty() {
            return;
        }
        Self::sanitize_samples(&mut samples);

        if let Some(sink) = self.active_sink.as_ref() {
            sink.append(SamplesBuffer::new(channels.max(1), sample_rate, samples));
        } else {
            self.play_samples_with_channels(samples, sample_rate, channels);
        }
    }

    fn sanitize_samples(samples: &mut [f32]) {
        for s in samples.iter_mut() {
            if !s.is_finite() {
                *s = 0.0;
            } else if s.abs() > 0.95 {
                let sign = s.signum();
                let excess = s.abs() - 0.90;
                let compressed = 0.90 + 0.08 * (excess / 0.08).tanh();
                *s = sign * compressed.min(0.98);
            }
        }
    }

    pub fn stop(&mut self) {
        if let Some(sink) = self.active_sink.take() {
            sink.pause();
            sink.detach();
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 2.0);
        if let Some(sink) = &self.active_sink {
            sink.set_volume(self.volume);
        }
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.25, 4.0);
        if let Some(sink) = &self.active_sink {
            sink.set_speed(self.speed);
        }
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn is_playing(&self) -> bool {
        if let Some(ref sink) = self.active_sink {
            return !sink.empty();
        }
        false
    }
}

#[cfg(target_arch = "wasm32")]
pub struct AudioPlayer {
    ctx: Option<web_sys::AudioContext>,
    scheduled_sources: Vec<web_sys::AudioBufferSourceNode>,
    next_play_time: f64,
    volume: f32,
    speed: f32,
    is_playing: bool,
}

#[cfg(target_arch = "wasm32")]
impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            ctx: None,
            scheduled_sources: Vec::new(),
            next_play_time: 0.0,
            volume: 1.0,
            speed: 1.0,
            is_playing: false,
        }
    }

    /// Web Audio must be resumed from the user gesture that requested
    /// playback. Calling this before rendering prevents autoplay policies
    /// from rejecting the first scheduled buffer after an async render.
    pub fn prepare_for_playback(&mut self) -> Result<(), String> {
        self.get_or_create_context().map(|_| ()).ok_or_else(|| {
            "O navegador não conseguiu inicializar o dispositivo de áudio Web Audio.".to_string()
        })
    }

    fn get_or_create_context(&mut self) -> Option<&web_sys::AudioContext> {
        if self.ctx.is_none() {
            if let Ok(ctx) = web_sys::AudioContext::new() {
                self.ctx = Some(ctx);
            }
        }
        if let Some(ref ctx) = self.ctx {
            let _ = ctx.resume();
        }
        self.ctx.as_ref()
    }

    pub fn list_output_devices() -> Vec<String> {
        vec!["Padrão do Navegador (Web Audio)".to_string()]
    }

    pub fn default_host_name() -> String {
        "Web Audio API".to_string()
    }

    pub fn play_samples(&mut self, samples: Vec<f32>, sample_rate: u32) {
        self.play_samples_with_channels(samples, sample_rate, 1);
    }

    pub fn play_samples_with_channels(
        &mut self,
        mut samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) {
        if samples.is_empty() {
            return;
        }
        Self::sanitize_samples(&mut samples);
        self.stop();

        let volume = self.volume;
        let speed = self.speed;

        if let Some(ctx) = self.get_or_create_context() {
            let num_channels = channels.max(1) as u32;
            let frame_count = (samples.len() / num_channels as usize) as u32;
            if frame_count == 0 {
                return;
            }
            if let Ok(buffer) = ctx.create_buffer(num_channels, frame_count, sample_rate as f32) {
                for ch in 0..num_channels {
                    let mut ch_data = Vec::with_capacity(frame_count as usize);
                    for i in 0..frame_count as usize {
                        let sample_idx = i * num_channels as usize + ch as usize;
                        if sample_idx < samples.len() {
                            ch_data.push(samples[sample_idx] * volume);
                        }
                    }
                    let _ = buffer.copy_to_channel(&ch_data, ch as i32);
                }
                if let Ok(source) = ctx.create_buffer_source() {
                    let _ = source.set_buffer(Some(&buffer));
                    source.playback_rate().set_value(speed);
                    let _ = source.connect_with_audio_node(&ctx.destination());
                    let now = ctx.current_time();
                    let dur = (frame_count as f64) / (sample_rate as f64);
                    let _ = source.start_with_when(now);
                    self.next_play_time = now + dur;
                    self.scheduled_sources.push(source);
                    self.is_playing = true;
                }
            }
        }
    }

    pub fn append_samples_with_channels(
        &mut self,
        mut samples: Vec<f32>,
        sample_rate: u32,
        channels: u16,
    ) {
        if samples.is_empty() {
            return;
        }
        if !self.is_playing || self.scheduled_sources.is_empty() {
            self.play_samples_with_channels(samples, sample_rate, channels);
            return;
        }
        Self::sanitize_samples(&mut samples);

        let volume = self.volume;
        let speed = self.speed;

        if let Some(ctx) = self.get_or_create_context() {
            let num_channels = channels.max(1) as u32;
            let frame_count = (samples.len() / num_channels as usize) as u32;
            if frame_count == 0 {
                return;
            }
            if let Ok(buffer) = ctx.create_buffer(num_channels, frame_count, sample_rate as f32) {
                for ch in 0..num_channels {
                    let mut ch_data = Vec::with_capacity(frame_count as usize);
                    for i in 0..frame_count as usize {
                        let sample_idx = i * num_channels as usize + ch as usize;
                        if sample_idx < samples.len() {
                            ch_data.push(samples[sample_idx] * volume);
                        }
                    }
                    let _ = buffer.copy_to_channel(&ch_data, ch as i32);
                }
                if let Ok(source) = ctx.create_buffer_source() {
                    let _ = source.set_buffer(Some(&buffer));
                    source.playback_rate().set_value(speed);
                    let _ = source.connect_with_audio_node(&ctx.destination());
                    let now = ctx.current_time();
                    let start_time = self.next_play_time.max(now);
                    let dur = (frame_count as f64) / (sample_rate as f64);
                    let _ = source.start_with_when(start_time);
                    self.next_play_time = start_time + dur;
                    self.scheduled_sources.push(source);
                }
            }
        }
    }

    fn sanitize_samples(samples: &mut [f32]) {
        for s in samples.iter_mut() {
            if !s.is_finite() {
                *s = 0.0;
            } else if s.abs() > 0.95 {
                let sign = s.signum();
                let excess = s.abs() - 0.90;
                let compressed = 0.90 + 0.08 * (excess / 0.08).tanh();
                *s = sign * compressed.min(0.98);
            }
        }
    }

    pub fn stop(&mut self) {
        self.is_playing = false;
        self.next_play_time = 0.0;
        for src in self.scheduled_sources.drain(..) {
            let _ = src.stop_with_when(0.0);
            let _ = src.disconnect();
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 2.0);
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed.clamp(0.25, 4.0);
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn is_playing(&self) -> bool {
        if !self.is_playing {
            return false;
        }
        if let Some(ref ctx) = self.ctx {
            ctx.current_time() < self.next_play_time
        } else {
            false
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl AudioPlayer {
    /// Desktop output is initialized by `new`; this keeps the playback call
    /// site identical across native and WebAssembly targets.
    pub fn prepare_for_playback(&mut self) -> Result<(), String> {
        if self.stream_handle.is_some() {
            Ok(())
        } else {
            Err("Nenhum dispositivo de saída de áudio está disponível.".to_string())
        }
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
        player.stop();
        assert!(!player.is_playing());
    }
}
