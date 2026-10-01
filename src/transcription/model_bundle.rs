//! Verified multi-file ONNX model bundles. A completion marker is published last.

use std::{collections::BTreeMap, fs, io::Read, path::Path};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::local_models::{
    download_model_with_handle, DownloadHandle, DownloadProgressCallback, RegistryEntry,
};

#[derive(Clone, Deserialize, Serialize)]
struct FileInfo {
    bytes: u64,
    sha256: String,
}

#[derive(Deserialize)]
struct Build {
    files: Vec<String>,
}

#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    model: Option<String>,
    builds: BTreeMap<String, Build>,
    files: BTreeMap<String, FileInfo>,
}

pub(crate) async fn resolve_custom(url: &reqwest::Url) -> anyhow::Result<RegistryEntry> {
    let variant = url.fragment().context("bundle URL must specify #variant")?;
    if !super::local_models::is_safe_model_id(variant) {
        bail!("invalid bundle variant");
    }
    let mut source = url.clone();
    source.set_fragment(None);
    let bytes = reqwest::get(source)
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    let build = manifest
        .builds
        .get(variant)
        .context("unknown bundle variant")?;
    let model = manifest
        .model
        .as_deref()
        .context("bundle manifest requires a model name")?;
    let id = format!(
        "{}-{variant}",
        model
            .rsplit('/')
            .next()
            .unwrap_or(model)
            .to_ascii_lowercase()
    );
    if !super::local_models::is_safe_model_id(&id) {
        bail!("manifest model name is not filesystem-safe");
    }
    let mut size = 0_u64;
    for name in
        build
            .files
            .iter()
            .map(String::as_str)
            .chain(["vocab.txt", "config.json", "nemo128.onnx"])
    {
        if !safe_filename(name) {
            bail!("unsafe model bundle filename: {name}");
        }
        size = size
            .checked_add(
                manifest
                    .files
                    .get(name)
                    .context("missing file checksum in manifest")?
                    .bytes,
            )
            .context("model bundle size overflow")?;
    }
    Ok(RegistryEntry {
        id,
        provider_id: "parakeet".to_string(),
        name: model.to_string(),
        description: "Custom Parakeet TDT ONNX bundle".to_string(),
        languages: Vec::new(),
        size_mb: size
            .div_ceil(1024 * 1024)
            .try_into()
            .context("model bundle is too large")?,
        url: url.to_string(),
        sha256: Some(format!("{:x}", Sha256::digest(&bytes))),
        recommended_hardware: None,
        category: None,
        group_id: Some("Custom".to_string()),
    })
}

#[derive(Deserialize, Serialize)]
struct Bundle {
    files: BTreeMap<String, FileInfo>,
}

pub(crate) fn catalog() -> Vec<RegistryEntry> {
    [ ("int8", 630, "CPU"), ("fp16", 1224, "Apple Silicon WebGPU / NVIDIA CUDA (experimental); CPU otherwise") ]
        .into_iter()
        .map(|(variant, size_mb, hardware)| RegistryEntry {
            id: format!("pianissimo-sv-{variant}"),
            provider_id: "parakeet".to_string(),
            name: format!("Pianissimo Swedish ({variant})"),
            description: "Klang AI Swedish Parakeet TDT fine-tune (CC BY 4.0)".to_string(),
            languages: vec!["sv".to_string()],
            size_mb,
            url: format!("https://huggingface.co/KlangAI/pianissimo-sv-onnx/resolve/63730c6021234f26b9bbae9a07a04fec39e7a52e/manifest.json#{variant}"),
            sha256: Some("b41b720dcf1d599ca41904e54475b953047035949e1463b47959550a1e0d5948".to_string()),
            recommended_hardware: Some(hardware.to_string()),
            category: None,
            group_id: Some("Parakeet".to_string()),
        }).collect()
}

fn safe_filename(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && name != "bundle.json"
        && !name.ends_with(".tmp")
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

fn stored_name(name: &str) -> &str {
    // parakeet-rs requires the canonical name for the FP16 decoder/joint graph.
    match name {
        "encoder-model.fp16.onnx" => "encoder-model.onnx",
        "decoder_joint-model.fp16.onnx" => "decoder_joint-model.onnx",
        _ => name,
    }
}

fn read_bundle(path: &Path) -> anyhow::Result<Bundle> {
    let bundle: Bundle = serde_json::from_slice(&fs::read(path.join("bundle.json"))?)?;
    if bundle.files.is_empty() || bundle.files.keys().any(|name| !safe_filename(name)) {
        bail!("invalid model bundle filenames");
    }
    Ok(bundle)
}

pub(crate) fn is_installed(path: &Path) -> bool {
    read_bundle(path).is_ok_and(|bundle| {
        bundle.files.iter().all(|(name, info)| {
            fs::metadata(path.join(name)).is_ok_and(|m| m.is_file() && m.len() == info.bytes)
        })
    })
}

fn verify_file(path: &Path, info: &FileInfo) -> anyhow::Result<()> {
    let mut file = fs::File::open(path)?;
    if file.metadata()?.len() != info.bytes {
        bail!("model file size mismatch: {}", path.display());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 65536];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if !format!("{:x}", hash.finalize()).eq_ignore_ascii_case(&info.sha256) {
        bail!("model checksum mismatch: {}", path.display());
    }
    Ok(())
}

pub(crate) fn validate(path: &Path) -> anyhow::Result<()> {
    for (name, info) in read_bundle(path)?.files {
        verify_file(&path.join(name), &info)?;
    }
    Ok(())
}

pub(crate) fn disk_usage(path: &Path) -> u64 {
    read_bundle(path)
        .map(|b| b.files.values().map(|f| f.bytes).sum())
        .unwrap_or(0)
}

pub(crate) fn remove(path: &Path) -> anyhow::Result<()> {
    let bundle = read_bundle(path)?;
    // Refuse to remove unrelated files, rather than recursively deleting the directory.
    for entry in fs::read_dir(path)? {
        let name = entry?.file_name().to_string_lossy().into_owned();
        if name != "bundle.json" && !bundle.files.contains_key(&name) {
            bail!("model directory contains an unrecognized file: {name}");
        }
    }
    for name in bundle.files.keys() {
        fs::remove_file(path.join(name))?;
    }
    fs::remove_file(path.join("bundle.json"))?;
    fs::remove_dir(path)?;
    Ok(())
}

pub(crate) async fn download(
    entry: &RegistryEntry,
    path: &Path,
    progress: Option<DownloadProgressCallback>,
    handle: Option<DownloadHandle>,
) -> anyhow::Result<()> {
    let progress = progress.map(std::sync::Arc::<dyn Fn(u64, u64, f64) + Send + Sync>::from);
    let (url, variant) = entry
        .url
        .split_once('#')
        .context("bundle URL must specify #variant")?;
    let bytes = reqwest::get(url).await?.error_for_status()?.bytes().await?;
    let expected = entry
        .sha256
        .as_deref()
        .context("bundle manifest requires a checksum")?;
    if !format!("{:x}", Sha256::digest(&bytes)).eq_ignore_ascii_case(expected) {
        bail!("model bundle manifest checksum mismatch");
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)?;
    let build = manifest
        .builds
        .get(variant)
        .context("unknown bundle variant")?;
    let mut sources = BTreeMap::new();
    let mut bundle = Bundle {
        files: BTreeMap::new(),
    };
    for name in
        build
            .files
            .iter()
            .map(String::as_str)
            .chain(["vocab.txt", "config.json", "nemo128.onnx"])
    {
        if !safe_filename(name) {
            bail!("unsafe model bundle filename: {name}");
        }
        let info = manifest
            .files
            .get(name)
            .context("missing file checksum in manifest")?;
        let stored = stored_name(name).to_string();
        if bundle.files.insert(stored.clone(), info.clone()).is_some() {
            bail!("duplicate model bundle destination: {stored}");
        }
        sources.insert(stored, name);
    }
    fs::create_dir_all(path)?;
    let base = url.rsplit_once('/').context("invalid bundle URL")?.0;
    let total = bundle.files.values().map(|f| f.bytes).sum();
    let mut completed = 0;
    for (stored, source) in sources {
        if handle.as_ref().is_some_and(DownloadHandle::is_cancelled) {
            bail!("model download cancelled");
        }
        let dest = path.join(&stored);
        let info = &bundle.files[&stored];
        if verify_file(&dest, info).is_err() {
            let callback: Option<DownloadProgressCallback> = progress.as_ref().map(|callback| {
                let callback = std::sync::Arc::clone(callback);
                Box::new(move |downloaded, _, speed| callback(completed + downloaded, total, speed))
                    as DownloadProgressCallback
            });
            download_model_with_handle(
                &format!("{base}/{source}"),
                &dest,
                callback,
                handle.clone(),
            )
            .await?;
            verify_file(&dest, info)?;
        }
        completed += info.bytes;
        if let Some(callback) = progress.as_ref() {
            callback(completed, total, 0.0);
        }
    }
    let marker = path.join("bundle.json.tmp");
    if handle.as_ref().is_some_and(DownloadHandle::is_cancelled) {
        bail!("model download cancelled");
    }
    fs::write(&marker, serde_json::to_vec(&bundle)?)?;
    fs::rename(marker, path.join("bundle.json"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifests_cannot_escape_the_model_directory() {
        for name in ["../encoder.onnx", "/encoder.onnx", "..", "a/b", "a\\b"] {
            assert!(!safe_filename(name), "{name}");
        }
        assert!(safe_filename("encoder-model.int8.onnx"));
    }

    #[test]
    fn fp16_files_use_the_names_required_by_the_runtime() {
        assert_eq!(
            stored_name("decoder_joint-model.fp16.onnx"),
            "decoder_joint-model.onnx"
        );
        assert_eq!(
            stored_name("decoder_joint-model.int8.onnx"),
            "decoder_joint-model.int8.onnx"
        );
    }

    #[test]
    fn partial_or_corrupt_downloads_are_not_verified_and_removal_preserves_unknown_files() {
        let path = std::env::temp_dir().join(format!(
            "ostt-bundle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        let data = b"fake model";
        let info = FileInfo {
            bytes: data.len() as u64,
            sha256: format!("{:x}", Sha256::digest(data)),
        };
        let bundle = Bundle {
            files: [("encoder.onnx".to_string(), info)].into(),
        };
        fs::write(path.join("encoder.onnx"), data).unwrap();
        assert!(
            !is_installed(&path),
            "no completion marker means no installed model"
        );
        fs::write(
            path.join("bundle.json"),
            serde_json::to_vec(&bundle).unwrap(),
        )
        .unwrap();
        assert!(is_installed(&path));
        validate(&path).unwrap();
        fs::write(path.join("encoder.onnx"), b"bad model").unwrap();
        assert!(
            validate(&path).is_err(),
            "equal-length corruption must fail checksum validation"
        );
        fs::write(path.join("encoder.onnx"), b"short").unwrap();
        assert!(
            !is_installed(&path),
            "truncated files must not appear installed"
        );
        fs::write(path.join("personal.txt"), b"keep").unwrap();
        assert!(remove(&path).is_err());
        assert!(
            path.join("encoder.onnx").exists(),
            "refuse removal before deleting any files"
        );
        assert_eq!(fs::read(path.join("personal.txt")).unwrap(), b"keep");
        fs::remove_file(path.join("personal.txt")).unwrap();
        remove(&path).unwrap();
        assert!(!path.exists());
    }
}
