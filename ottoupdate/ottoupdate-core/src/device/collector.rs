use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sysinfo::{Disks, Networks, System};
use tokio::fs;

const MIN_SAFE_BATTERY_PERCENT: u8 = 20;
const MIN_SAFE_DISK_FREE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct DeviceStateCollector {
    config: DeviceStateCollectorConfig,
}

#[derive(Debug, Clone)]
pub struct DeviceStateCollectorConfig {
    pub data_dir: PathBuf,
    pub install_manifest_path: PathBuf,
    pub device_tags_path: PathBuf,
    pub update_history_path: PathBuf,
    pub output_path: PathBuf,
}

impl Default for DeviceStateCollectorConfig {
    fn default() -> Self {
        let data_dir = PathBuf::from("data");
        Self {
            install_manifest_path: data_dir.join("install_manifest.json"),
            device_tags_path: data_dir.join("device_tags.json"),
            update_history_path: data_dir.join("update_history.json"),
            output_path: data_dir.join("device_state.json"),
            data_dir,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceState {
    pub device_id: String,
    pub hostname: String,
    pub recorded_at: DateTime<Utc>,
    pub platform: PlatformState,
    pub hardware: HardwareState,
    pub network: NetworkState,
    pub installed_product: InstalledProductState,
    pub update_history: Vec<UpdateHistoryItem>,
    pub tags: Vec<String>,
    pub deferred_count: u32,
    pub last_deferred_at: Option<DateTime<Utc>>,
    pub managed: bool,
    pub management_group: Option<String>,
}

impl DeviceState {
    pub fn is_safe_to_update(&self) -> bool {
        let has_network = self.network.connected;
        let has_disk = self.hardware.disk_free_bytes >= MIN_SAFE_DISK_FREE_BYTES;
        let battery_ok = self.hardware.on_ac_power
            || self
                .hardware
                .battery_percent
                .map(|percent| percent >= MIN_SAFE_BATTERY_PERCENT)
                .unwrap_or(true);

        has_network && has_disk && battery_ok
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct PlatformState {
    pub os: String,
    pub os_version: String,
    pub kernel_version: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HardwareState {
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub ram_bytes: u64,
    pub disk_total_bytes: u64,
    pub disk_free_bytes: u64,
    pub battery_percent: Option<u8>,
    pub on_ac_power: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct NetworkState {
    pub connected: bool,
    pub connection_type: String,
    pub metered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct InstalledProductState {
    pub name: String,
    pub version: String,
    pub channel: String,
    pub install_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateHistoryItem {
    pub event_id: String,
    pub version: String,
    pub outcome: String,
    pub recorded_at: DateTime<Utc>,
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InstallManifestFile {
    name: Option<String>,
    version: Option<String>,
    channel: Option<String>,
    install_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeviceTagsFile {
    tags: Option<Vec<String>>,
    deferred_count: Option<u32>,
    last_deferred_at: Option<DateTime<Utc>>,
    managed: Option<bool>,
    management_group: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum UpdateHistoryFile {
    Wrapped { items: Vec<UpdateHistoryItem> },
    Flat(Vec<UpdateHistoryItem>),
}

impl DeviceStateCollector {
    pub fn new(config: DeviceStateCollectorConfig) -> Self {
        Self { config }
    }

    pub async fn collect() -> Result<DeviceState> {
        Self::new(DeviceStateCollectorConfig::default())
            .collect_with_config()
            .await
    }

    pub async fn collect_with_config(&self) -> Result<DeviceState> {
        fs::create_dir_all(&self.config.data_dir)
            .await
            .with_context(|| {
                format!(
                    "failed to create data dir {}",
                    self.config.data_dir.display()
                )
            })?;

        let recorded_at = Utc::now();
        let device_id = read_or_create_device_id(&self.config.data_dir).await?;
        let hostname = read_hostname();

        let platform = collect_platform_state();
        let hardware = collect_hardware_state().await?;
        let network = collect_network_state().await?;
        let installed_product = read_installed_product(&self.config.install_manifest_path).await?;

        let (tags, deferred_count, last_deferred_at, managed, management_group) =
            read_device_tags(&self.config.device_tags_path).await?;
        let update_history = read_update_history(&self.config.update_history_path).await?;

        let state = DeviceState {
            device_id,
            hostname,
            recorded_at,
            platform,
            hardware,
            network,
            installed_product,
            update_history,
            tags,
            deferred_count,
            last_deferred_at,
            managed,
            management_group,
        };

        persist_device_state(&state, &self.config.output_path).await?;

        Ok(state)
    }
}

fn collect_platform_state() -> PlatformState {
    let os = if cfg!(target_os = "linux") {
        "linux".to_string()
    } else if cfg!(target_os = "macos") {
        "macos".to_string()
    } else if cfg!(target_os = "windows") {
        "windows".to_string()
    } else {
        "unknown".to_string()
    };

    let arch = if cfg!(target_arch = "x86") {
        "x86".to_string()
    } else if cfg!(target_arch = "x86_64") {
        "x64".to_string()
    } else if cfg!(target_arch = "aarch64") {
        "arm64".to_string()
    } else {
        "unknown".to_string()
    };

    PlatformState {
        os,
        os_version: System::long_os_version().unwrap_or_else(|| "unknown".to_string()),
        kernel_version: System::kernel_version().unwrap_or_else(|| "unknown".to_string()),
        arch,
    }
}

async fn collect_hardware_state() -> Result<HardwareState> {
    let (cpu_model, cpu_cores, ram_bytes, disk_total_bytes, disk_free_bytes) =
        tokio::task::spawn_blocking(move || {
            let mut system = System::new_all();
            system.refresh_all();

            let cpu_model = system
                .cpus()
                .first()
                .map(|cpu| cpu.brand().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "unknown".to_string());
            let cpu_cores = system.cpus().len() as u32;

            let ram_bytes = system.total_memory();

            let disks = Disks::new_with_refreshed_list();
            let (disk_total_bytes, disk_free_bytes) = disks.iter().fold((0_u64, 0_u64), |acc, disk| {
                (acc.0 + disk.total_space(), acc.1 + disk.available_space())
            });

            (
                cpu_model,
                cpu_cores,
                ram_bytes,
                disk_total_bytes,
                disk_free_bytes,
            )
        })
        .await
        .context("hardware collection task failed")?;

    let (battery_percent, on_ac_power) = collect_power_state().await;

    Ok(HardwareState {
        cpu_model,
        cpu_cores,
        ram_bytes,
        disk_total_bytes,
        disk_free_bytes,
        battery_percent,
        on_ac_power,
    })
}

async fn collect_network_state() -> Result<NetworkState> {
    let (connected, connection_type) = tokio::task::spawn_blocking(move || {
        let networks = Networks::new_with_refreshed_list();
        if networks.is_empty() {
            return (false, "unknown".to_string());
        }

        let mut interface_names = Vec::new();
        for (name, _) in &networks {
            interface_names.push(name.to_lowercase());
        }

        let connection_type = infer_connection_type(&interface_names);
        (true, connection_type)
    })
    .await
    .context("network collection task failed")?;

    Ok(NetworkState {
        connected,
        connection_type,
        metered: detect_metered_connection(),
    })
}

fn infer_connection_type(interface_names: &[String]) -> String {
    for name in interface_names {
        if name.contains("wi") || name.contains("wlan") || name.contains("wifi") {
            return "wifi".to_string();
        }
        if name.contains("eth") || name.starts_with("en") {
            return "ethernet".to_string();
        }
        if name.contains("wwan") || name.contains("cell") || name.contains("rmnet") {
            return "cellular".to_string();
        }
    }

    "unknown".to_string()
}

#[cfg(target_os = "windows")]
fn detect_metered_connection() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
fn detect_metered_connection() -> bool {
    false
}

async fn collect_power_state() -> (Option<u8>, bool) {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        if let Some(value) = collect_power_state_battery_crate().await {
            return value;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(value) = collect_power_state_linux_sysfs().await {
            return value;
        }
    }

    (None, true)
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
async fn collect_power_state_battery_crate() -> Option<(Option<u8>, bool)> {
    tokio::task::spawn_blocking(move || {
        let manager = battery::Manager::new().ok()?;
        let mut batteries = manager.batteries().ok()?;
        let battery = batteries.next()?.ok()?;

        let percentage = (battery.state_of_charge().value * 100.0).round();
        let percentage = percentage.clamp(0.0, 100.0) as u8;

        let on_ac_power = matches!(battery.state(), battery::State::Charging | battery::State::Full);

        Some((Some(percentage), on_ac_power))
    })
    .await
    .ok()
    .flatten()
}

#[cfg(target_os = "linux")]
async fn collect_power_state_linux_sysfs() -> Option<(Option<u8>, bool)> {
    let mut battery_percent: Option<u8> = None;
    let mut ac_online = false;

    let mut dir = fs::read_dir("/sys/class/power_supply").await.ok()?;
    while let Ok(Some(entry)) = dir.next_entry().await {
        let path = entry.path();
        let Some(power_type) = read_trimmed(path.join("type")).await else {
            continue;
        };

        if power_type == "Battery" {
            if let Some(capacity) = read_trimmed(path.join("capacity")).await {
                if let Ok(value) = capacity.parse::<u8>() {
                    battery_percent = Some(value.min(100));
                }
            }
        }

        if power_type == "Mains" || power_type == "AC" {
            if let Some(online) = read_trimmed(path.join("online")).await {
                ac_online = online == "1";
            }
        }
    }

    Some((battery_percent, ac_online || battery_percent.is_none()))
}

async fn read_installed_product(path: &Path) -> Result<InstalledProductState> {
    let manifest = read_json_optional::<InstallManifestFile>(path).await?;

    Ok(InstalledProductState {
        name: manifest
            .as_ref()
            .and_then(|x| x.name.clone())
            .unwrap_or_else(|| "otto".to_string()),
        version: manifest
            .as_ref()
            .and_then(|x| x.version.clone())
            .unwrap_or_else(|| "0.0.0".to_string()),
        channel: manifest
            .as_ref()
            .and_then(|x| x.channel.clone())
            .unwrap_or_else(|| "stable".to_string()),
        install_path: manifest
            .as_ref()
            .and_then(|x| x.install_path.clone())
            .unwrap_or_else(|| "unknown".to_string()),
    })
}

async fn read_device_tags(
    path: &Path,
) -> Result<(Vec<String>, u32, Option<DateTime<Utc>>, bool, Option<String>)> {
    let tags_file = read_json_optional::<DeviceTagsFile>(path).await?;
    let Some(tags_file) = tags_file else {
        return Ok((Vec::new(), 0, None, true, None));
    };

    let mut deduped = Vec::new();
    let mut seen = HashSet::new();

    for tag in tags_file.tags.unwrap_or_default() {
        let normalized = tag.trim();
        if normalized.is_empty() {
            continue;
        }
        if seen.insert(normalized.to_string()) {
            deduped.push(normalized.to_string());
        }
    }

    Ok((
        deduped,
        tags_file.deferred_count.unwrap_or(0),
        tags_file.last_deferred_at,
        tags_file.managed.unwrap_or(true),
        tags_file.management_group,
    ))
}

async fn read_update_history(path: &Path) -> Result<Vec<UpdateHistoryItem>> {
    let parsed = read_json_optional::<UpdateHistoryFile>(path).await?;
    let items = match parsed {
        Some(UpdateHistoryFile::Wrapped { items }) => items,
        Some(UpdateHistoryFile::Flat(items)) => items,
        None => Vec::new(),
    };

    Ok(items)
}

async fn read_or_create_device_id(data_dir: &Path) -> Result<String> {
    let id_path = data_dir.join("device_id.txt");

    match fs::read_to_string(&id_path).await {
        Ok(content) => {
            let id = content.trim().to_string();
            if id.is_empty() {
                create_device_id_file(&id_path).await
            } else {
                Ok(id)
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => create_device_id_file(&id_path).await,
        Err(err) => Err(err).with_context(|| format!("failed to read {}", id_path.display())),
    }
}

async fn create_device_id_file(path: &Path) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    fs::write(path, format!("{id}\n"))
        .await
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(id)
}

fn read_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|name| name.into_string().ok())
        .unwrap_or_else(|| "unknown-host".to_string())
}

async fn persist_device_state(state: &DeviceState, output_path: &Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let serialized = serde_json::to_string_pretty(state).context("failed to serialize device state")?;
    fs::write(output_path, serialized)
        .await
        .with_context(|| format!("failed to write {}", output_path.display()))?;

    Ok(())
}

async fn read_json_optional<T>(path: &Path) -> Result<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    match fs::read_to_string(path).await {
        Ok(content) => {
            let parsed = serde_json::from_str::<T>(&content)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            Ok(Some(parsed))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err).with_context(|| format!("failed to read {}", path.display())),
    }
}

async fn read_trimmed(path: PathBuf) -> Option<String> {
    let raw = fs::read_to_string(path).await.ok()?;
    Some(raw.trim().to_string())
}
