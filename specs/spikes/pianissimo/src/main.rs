use anyhow::{bail, Context, Result};
use parakeet_rs::{ExecutionConfig, ParakeetTDT, Transcriber};
use std::{env, path::Path, time::Instant};

fn load_audio(path: &Path) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path).context("opening audio")?;
    let spec = reader.spec();
    if spec.sample_rate != 16000
        || spec.channels != 1
        || spec.sample_format != hound::SampleFormat::Int
        || spec.bits_per_sample != 16
    {
        bail!("expected 16 kHz mono PCM16 WAV; convert with ffmpeg first");
    }
    let samples = reader
        .samples::<i16>()
        .map(|s| s.map(|s| f32::from(s) / 32768.0))
        .collect::<Result<Vec<_>, _>>()?;
    if samples.is_empty() {
        bail!("audio is empty");
    }
    Ok(samples)
}

fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 {
        bail!("usage: pianissimo-spike MODEL_DIRECTORY AUDIO.wav");
    }
    let samples = load_audio(Path::new(&args[1]))?;
    let duration = samples.len() as f64 / 16000.0;
    eprintln!("provider=CPU threads=4 audio_seconds={duration:.3}");
    let started = Instant::now();
    let mut model = ParakeetTDT::from_pretrained(&args[0], Some(ExecutionConfig::default()))
        .context("loading Pianissimo TDT model")?;
    let load_seconds = started.elapsed().as_secs_f64();
    eprintln!("model_load_seconds={load_seconds:.3}");
    let mut previous = None;
    for run in 1..=3 {
        let started = Instant::now();
        let result = model
            .transcribe_samples(samples.clone(), 16000, 1, None)
            .with_context(|| format!("inference run {run}"))?;
        let elapsed = started.elapsed().as_secs_f64();
        if result.text.trim().is_empty() {
            bail!("empty transcript on speech fixture");
        }
        if previous.as_ref().is_some_and(|text| text != &result.text) {
            bail!("transcript changed between identical repeated inputs");
        }
        eprintln!(
            "run={run} inference_seconds={elapsed:.3} realtime_factor={:.4}",
            elapsed / duration
        );
        println!("{}", result.text);
        previous = Some(result.text);
    }
    Ok(())
}
