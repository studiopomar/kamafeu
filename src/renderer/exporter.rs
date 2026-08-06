use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub struct AudioExporter;

impl AudioExporter {
    /// Export f32 PCM audio buffer to 16-bit WAV file on disk
    pub fn export_to_wav<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
    ) -> Result<(), String> {
        let file =
            File::create(path.as_ref()).map_err(|e| format!("Failed to create WAV file: {}", e))?;
        let mut writer = BufWriter::new(file);

        let num_channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * u32::from(num_channels) * u32::from(bits_per_sample / 8);
        let block_align = num_channels * (bits_per_sample / 8);
        let data_size = samples.len() as u32 * 2;
        let chunk_size = 36 + data_size;

        // RIFF header
        writer.write_all(b"RIFF").map_err(|e| e.to_string())?;
        writer
            .write_all(&chunk_size.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer.write_all(b"WAVE").map_err(|e| e.to_string())?;

        // fmt chunk
        writer.write_all(b"fmt ").map_err(|e| e.to_string())?;
        writer
            .write_all(&16u32.to_le_bytes())
            .map_err(|e| e.to_string())?; // Subchunk1Size (16 for PCM)
        writer
            .write_all(&1u16.to_le_bytes())
            .map_err(|e| e.to_string())?; // AudioFormat (1 for PCM)
        writer
            .write_all(&num_channels.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&sample_rate.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&byte_rate.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&block_align.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&bits_per_sample.to_le_bytes())
            .map_err(|e| e.to_string())?;

        // data chunk
        writer.write_all(b"data").map_err(|e| e.to_string())?;
        writer
            .write_all(&data_size.to_le_bytes())
            .map_err(|e| e.to_string())?;

        for &sample in samples {
            let clamped = sample.clamp(-1.0, 1.0);
            let s_i16 = (clamped * 32767.0) as i16;
            writer
                .write_all(&s_i16.to_le_bytes())
                .map_err(|e| e.to_string())?;
        }

        writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}
