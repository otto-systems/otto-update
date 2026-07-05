use std::path::Path;

use async_trait::async_trait;
use anyhow::{anyhow, Result};
use base64::Engine;
use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
use sha2::{Digest, Sha256};
use tokio::fs;
use tracing::{instrument, warn};

use crate::traits::Artifact;

#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(&self, path: &Path, artifact: &Artifact) -> Result<()>;
}

pub struct Ed25519Verifier {
    pub require_signatures: bool,
}

impl Default for Ed25519Verifier {
    fn default() -> Self {
        Self {
            require_signatures: false,
        }
    }
}

impl Ed25519Verifier {
    fn decode_hex(bytes_hex: &str, label: &str) -> Result<Vec<u8>> {
        hex::decode(bytes_hex).map_err(|err| anyhow!("invalid {label} hex: {err}"))
    }

    fn decode_b64(bytes_b64: &str, label: &str) -> Result<Vec<u8>> {
        base64::engine::general_purpose::STANDARD
            .decode(bytes_b64)
            .map_err(|err| anyhow!("invalid {label} base64: {err}"))
    }
}

#[async_trait]
impl Verifier for Ed25519Verifier {
    #[instrument(skip(self), fields(artifact_id = %artifact.id, path = %path.display()))]
    async fn verify(&self, path: &Path, artifact: &Artifact) -> Result<()> {
        let bytes = fs::read(path)
            .await
            .map_err(|err| anyhow!("failed to read artifact file: {err}"))?;

        if let Some(expected_sha256) = artifact.sha256_hex.as_ref() {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual = hex::encode(hasher.finalize());
            if &actual != expected_sha256 {
                return Err(anyhow!(
                    "sha256 mismatch: expected {}, got {}",
                    expected_sha256,
                    actual
                ));
            }
        }

        let Some(signature_b64) = artifact.signature_b64.as_ref() else {
            if self.require_signatures {
                return Err(anyhow!("missing artifact signature"));
            }
            warn!("artifact signature missing; verification skipped (not required)");
            return Ok(());
        };

        let public_key_hex = artifact
            .public_key_hex
            .as_ref()
            .ok_or_else(|| anyhow!("missing artifact public key"))?;

        let signature_bytes = Self::decode_b64(signature_b64, "signature")?;
        let public_key_bytes = Self::decode_hex(public_key_hex, "public key")?;

        let signature = Signature::try_from(signature_bytes.as_slice())
            .map_err(|err| anyhow!("invalid signature format: {err}"))?;
        let key_array: [u8; 32] = public_key_bytes
            .try_into()
            .map_err(|_| anyhow!("public key must be 32 bytes"))?;
        let verifying_key = VerifyingKey::from_bytes(&key_array)
            .map_err(|err| anyhow!("invalid public key: {err}"))?;

        verifying_key
            .verify(&bytes, &signature)
            .map_err(|err| anyhow!("ed25519 verification failed: {err}"))?;

        Ok(())
    }
}
