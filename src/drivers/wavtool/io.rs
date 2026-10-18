use std::path::Path;

pub(super) fn load_wavtool_output(
    note_samples: &mut [f32],
    expected_sample_rate: u32,
    output_wav: &Path,
) -> Result<(), String> {
    if !output_wav.is_file() {
        return Err("o wavtool não criou o WAV de saída".to_string());
    }

    let (mut output, output_sample_rate) =
        crate::renderer::TrackRenderer::load_wav_samples(output_wav)?;
    if output.is_empty() {
        return Err("o wavtool criou um WAV vazio".to_string());
    }
    if output_sample_rate != expected_sample_rate {
        output = crate::renderer::TrackRenderer::convert_sample_rate(
            &output,
            output_sample_rate,
            expected_sample_rate,
        );
    }

    note_samples.fill(0.0);
    let copied = note_samples.len().min(output.len());
    note_samples[..copied].copy_from_slice(&output[..copied]);
    Ok(())
}
