use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub trait Validate {
    fn validate(&self) -> Result<(), ManifestValidationError>;
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ManifestValidationError {
    #[error("expires_at must be after released_at")]
    InvalidExpiryWindow,

    #[error("version semver must begin with major.minor.patch")]
    SemverMismatch,

    #[error("revoked=true requires revocation details")]
    MissingRevocation,

    #[error("revocation details are not allowed when revoked=false")]
    UnexpectedRevocation,

    #[error("no artifact matches current platform and architecture")]
    NoCompatibleArtifact,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseManifest {
    pub schema_version: String,
    pub manifest_id: Uuid,
    pub product: String,
    pub channel: ReleaseChannel,
    pub released_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub version: VersionInfo,
    pub artifacts: Vec<Artifact>,
    pub release_notes: ReleaseNotes,
    pub rollout: Rollout,
    pub dependencies: Option<Vec<Dependency>>,
    pub revoked: bool,
    pub revocation: Option<Revocation>,
}

impl ReleaseManifest {
    pub fn current_artifact(&self) -> Option<&Artifact> {
        let platform = current_platform();
        let arch = current_arch();

        self.artifacts
            .iter()
            .find(|artifact| artifact.platform == platform && artifact.arch == arch)
    }
}

impl Validate for ReleaseManifest {
    fn validate(&self) -> Result<(), ManifestValidationError> {
        if let Some(expires_at) = self.expires_at {
            if expires_at <= self.released_at {
                return Err(ManifestValidationError::InvalidExpiryWindow);
            }
        }

        let expected_prefix = format!(
            "{}.{}.{}",
            self.version.major, self.version.minor, self.version.patch
        );
        if !self.version.semver.starts_with(&expected_prefix) {
            return Err(ManifestValidationError::SemverMismatch);
        }

        if self.revoked && self.revocation.is_none() {
            return Err(ManifestValidationError::MissingRevocation);
        }

        if !self.revoked && self.revocation.is_some() {
            return Err(ManifestValidationError::UnexpectedRevocation);
        }

        if self.current_artifact().is_none() {
            return Err(ManifestValidationError::NoCompatibleArtifact);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseChannel {
    Stable,
    Beta,
    Canary,
    Lts,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct VersionInfo {
    pub semver: String,
    pub build: String,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Artifact {
    pub artifact_id: String,
    pub platform: Platform,
    pub arch: Architecture,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub signature: String,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Linux,
    Macos,
    Windows,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X64,
    Arm64,
    X86,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseNotes {
    pub summary: String,
    pub highlights: Vec<String>,
    pub breaking_changes: Vec<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Rollout {
    pub strategy: RolloutStrategy,
    pub staged_percentage: u8,
    pub canary_groups: Vec<String>,
    pub start_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RolloutStrategy {
    Immediate,
    Staged,
    Canary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Dependency {
    pub name: String,
    pub constraint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Revocation {
    pub reason: String,
    pub revoked_at: DateTime<Utc>,
    pub revoked_by: String,
}

fn current_platform() -> Platform {
    match std::env::consts::OS {
        "linux" => Platform::Linux,
        "macos" => Platform::Macos,
        "windows" => Platform::Windows,
        _ => Platform::Linux,
    }
}

fn current_arch() -> Architecture {
    match std::env::consts::ARCH {
        "x86" | "i686" => Architecture::X86,
        "x86_64" => Architecture::X64,
        "aarch64" => Architecture::Arm64,
        _ => Architecture::X64,
    }
}
