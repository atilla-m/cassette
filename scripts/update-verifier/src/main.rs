//! Offline verification of the exact format used by tauri-plugin-updater 2.11.
//! Same base64 decoding and minisign_verify::PublicKey::verify(..., true).
//! No signing capability, private key, network client, or install operation.
use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use std::{fs, path::Path};

fn verify(
    image: &Path,
    signature: &Path,
    config: &Path,
    version: &str,
    windows: bool,
) -> Result<(), String> {
    let image_name = if windows {
        format!("Cassette_{version}_x64-setup.exe")
    } else {
        format!("Cassette_{version}_amd64.AppImage")
    };
    if image.file_name().and_then(|s| s.to_str()) != Some(&image_name)
        || signature.file_name().and_then(|s| s.to_str()) != Some(&format!("{image_name}.sig"))
        || signature.parent() != image.parent()
    {
        return Err("Update payload and signature filenames or directories do not match".into());
    }
    let configuration: serde_json::Value =
        serde_json::from_slice(&fs::read(config).map_err(|_| "Cannot read configuration")?)
            .map_err(|_| "Invalid configuration JSON")?;
    let key = configuration
        .pointer("/plugins/updater/pubkey")
        .and_then(|v| v.as_str())
        .ok_or("Configured public verification key is required")?;
    let key_bytes = STANDARD
        .decode(key.trim())
        .map_err(|_| "Malformed public key encoding")?;
    let public =
        PublicKey::decode(std::str::from_utf8(&key_bytes).map_err(|_| "Invalid public key text")?)
            .map_err(|_| "Invalid public verification key")?;
    if fs::metadata(signature)
        .map_err(|_| "Missing signature")?
        .len()
        > 4096
    {
        return Err("Oversized signature".into());
    }
    let signature_text = fs::read_to_string(signature).map_err(|_| "Cannot read signature")?;
    let signature_bytes = STANDARD
        .decode(signature_text.trim())
        .map_err(|_| "Malformed signature encoding")?;
    let signature = Signature::decode(
        std::str::from_utf8(&signature_bytes).map_err(|_| "Invalid signature text")?,
    )
    .map_err(|_| "Invalid signature")?;
    if !signature
        .trusted_comment()
        .split('\t')
        .any(|s| s == format!("file:{image_name}"))
    {
        return Err("Signed filename does not match update payload".into());
    }
    let payload = fs::read(image).map_err(|_| "Cannot read update payload")?;
    public
        .verify(&payload, &signature, true)
        .map_err(|_| "Update payload signature verification FAILED".into())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let windows = args.get(4).is_some_and(|flag| flag == "--windows");
    let result = if args.len() == 4 || (args.len() == 5 && windows) {
        verify(
            Path::new(&args[0]),
            Path::new(&args[1]),
            Path::new(&args[2]),
            &args[3],
            windows,
        )
    } else {
        Err("Usage: cassette-update-verifier PAYLOAD SIGNATURE CONFIG VERSION [--windows]".into())
    };
    match result {
        Ok(()) => println!("Update signature verified with the configured public key."),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
