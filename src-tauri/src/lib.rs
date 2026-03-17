use brtt::{Rtt, ScanRegion};

use probe_rs::config::TargetSelector;
use probe_rs::flashing::{download_file, image_format};
use probe_rs::probe::list::Lister;
use probe_rs::probe::DebugProbeInfo;
use probe_rs::{Permissions, Session};

use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

// ── Types for the frontend ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct ProbeInfo {
    pub index: usize,
    pub identifier: String,
    pub vendor_id: u16,
    pub product_id: u16,
    pub serial_number: Option<String>,
}

impl From<(usize, &DebugProbeInfo)> for ProbeInfo {
    fn from((index, info): (usize, &DebugProbeInfo)) -> Self {
        Self {
            index,
            identifier: info.identifier.clone(),
            vendor_id: info.vendor_id,
            product_id: info.product_id,
            serial_number: info.serial_number.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RttChannelInfo {
    pub number: usize,
    pub name: Option<String>,
    pub buffer_size: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlashProgress {
    pub phase: String,
    pub progress: f64,
    pub message: String,
}

// ── App state ───────────────────────────────────────────────────────────────

pub struct AppState {
    session: Mutex<Option<Session>>,
    rtt: Mutex<Option<RttState>>,
}

struct RttState {
    rtt: Rtt,
    up_channel: usize,
    down_channel: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
            rtt: Mutex::new(None),
        }
    }
}

// ── Tauri commands ──────────────────────────────────────────────────────────

#[tauri::command]
fn list_probes() -> Vec<ProbeInfo> {
    let lister = Lister::new();
    lister
        .list_all()
        .iter()
        .enumerate()
        .map(ProbeInfo::from)
        .collect()
}

#[tauri::command]
fn list_chips() -> Vec<String> {
    let registry = probe_rs::config::Registry::from_builtin_families();
    let mut chips: Vec<String> = registry
        .families()
        .iter()
        .flat_map(|family| family.variants.iter().map(|v| v.name.clone()))
        .collect();
    chips.sort();
    chips
}

#[tauri::command]
fn connect(
    state: State<'_, AppState>,
    probe_index: usize,
    chip: Option<String>,
) -> Result<String, String> {
    let lister = Lister::new();
    let probes = lister.list_all();

    if probe_index >= probes.len() {
        return Err(format!(
            "Probe index {} out of range (found {} probes)",
            probe_index,
            probes.len()
        ));
    }

    let probe = probes[probe_index].open().map_err(|e| e.to_string())?;
    let target_selector = TargetSelector::from(chip.as_deref());

    let session = probe
        .attach(target_selector, Permissions::default())
        .map_err(|e| format!("Failed to attach: {e}"))?;

    let target_name = session.target().name.clone();
    *state.session.lock() = Some(session);

    Ok(target_name)
}

#[tauri::command]
fn disconnect(state: State<'_, AppState>) {
    *state.rtt.lock() = None;
    *state.session.lock() = None;
}

#[tauri::command]
fn flash_firmware(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    let mut session_guard = state.session.lock();
    let session = session_guard
        .as_mut()
        .ok_or("Not connected to a probe")?;

    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("File not found: {path}"));
    }

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| format!("Cannot determine file extension for: {path}"))?;

    let format = image_format(ext)
        .ok_or_else(|| format!("Unknown file format: {ext}"))?
        .create_loader(None);

    download_file(session, &file_path, format)
        .map_err(|e| format!("Flash failed: {e}"))?;

    Ok(format!("Successfully flashed {path}"))
}

#[tauri::command]
fn rtt_attach(
    state: State<'_, AppState>,
    up_channel: Option<usize>,
    down_channel: Option<usize>,
    scan_region: Option<String>,
) -> Result<Vec<RttChannelInfo>, String> {
    let mut session_guard = state.session.lock();
    let session = session_guard
        .as_mut()
        .ok_or("Not connected to a probe")?;

    let mut core = session.core(0).map_err(|e| format!("Failed to get core: {e}"))?;

    let region = if let Some(ref region_str) = scan_region {
        parse_scan_region(region_str).map_err(|e| format!("Invalid scan region: {e}"))?
    } else {
        ScanRegion::Ram
    };

    let mut rtt = Rtt::attach_region(&mut core, &region)
        .map_err(|e| format!("Failed to attach RTT: {e}"))?;

    let channels: Vec<RttChannelInfo> = rtt
        .up_channels()
        .iter()
        .map(|ch| RttChannelInfo {
            number: ch.number(),
            name: ch.name().map(String::from),
            buffer_size: ch.buffer_size(),
        })
        .collect();

    drop(core);

    *state.rtt.lock() = Some(RttState {
        rtt,
        up_channel: up_channel.unwrap_or(0),
        down_channel: down_channel.unwrap_or(0),
    });

    Ok(channels)
}

#[tauri::command]
fn rtt_read(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let mut session_guard = state.session.lock();
    let session = session_guard
        .as_mut()
        .ok_or("Not connected to a probe")?;

    let mut rtt_guard = state.rtt.lock();
    let rtt_state = rtt_guard.as_mut().ok_or("RTT not attached")?;

    let mut core = session
        .core(0)
        .map_err(|e| format!("Failed to get core: {e}"))?;

    let mut buf = [0u8; 1024];

    if let Some(up_ch) = rtt_state.rtt.up_channel(rtt_state.up_channel) {
        let count = up_ch
            .read(&mut core, &mut buf)
            .map_err(|e| format!("RTT read error: {e}"))?;

        if count > 0 {
            Ok(Some(String::from_utf8_lossy(&buf[..count]).to_string()))
        } else {
            Ok(None)
        }
    } else {
        Err(format!(
            "Up channel {} not found",
            rtt_state.up_channel
        ))
    }
}

#[tauri::command]
fn rtt_write(state: State<'_, AppState>, data: String) -> Result<usize, String> {
    let mut session_guard = state.session.lock();
    let session = session_guard
        .as_mut()
        .ok_or("Not connected to a probe")?;

    let mut rtt_guard = state.rtt.lock();
    let rtt_state = rtt_guard.as_mut().ok_or("RTT not attached")?;

    let mut core = session
        .core(0)
        .map_err(|e| format!("Failed to get core: {e}"))?;

    if let Some(down_ch) = rtt_state.rtt.down_channel(rtt_state.down_channel) {
        let count = down_ch
            .write(&mut core, data.as_bytes())
            .map_err(|e| format!("RTT write error: {e}"))?;
        Ok(count)
    } else {
        Err(format!(
            "Down channel {} not found",
            rtt_state.down_channel
        ))
    }
}

#[tauri::command]
fn rtt_detach(state: State<'_, AppState>) {
    *state.rtt.lock() = None;
}

#[tauri::command]
fn reset_target(state: State<'_, AppState>) -> Result<(), String> {
    let mut session_guard = state.session.lock();
    let session = session_guard
        .as_mut()
        .ok_or("Not connected to a probe")?;

    let mut core = session
        .core(0)
        .map_err(|e| format!("Failed to get core: {e}"))?;

    core.reset().map_err(|e| format!("Reset failed: {e}"))
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn parse_scan_region(
    src: &str,
) -> Result<ScanRegion, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let src = src.trim();
    if src.is_empty() {
        return Ok(ScanRegion::Ram);
    }

    let parts = src
        .split("..")
        .map(|p| {
            if p.starts_with("0x") || p.starts_with("0X") {
                u64::from_str_radix(&p[2..], 16)
            } else {
                p.parse()
            }
        })
        .collect::<Result<Vec<_>, _>>()?;

    match *parts.as_slice() {
        [addr] => Ok(ScanRegion::Exact(addr)),
        [start, end] => Ok(ScanRegion::range(start..end)),
        _ => Err("Invalid range: multiple '..'s".into()),
    }
}

// ── Main ────────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            list_probes,
            list_chips,
            connect,
            disconnect,
            flash_firmware,
            rtt_attach,
            rtt_read,
            rtt_write,
            rtt_detach,
            reset_target,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
