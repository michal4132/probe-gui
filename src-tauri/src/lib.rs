use brtt::{Rtt, ScanRegion};

use probe_rs::config::TargetSelector;
use probe_rs::flashing::{download_file, image_format};
use probe_rs::probe::list::Lister;
use probe_rs::probe::DebugProbeInfo;
use probe_rs::{Permissions, Session};

use parking_lot::Mutex;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, State};

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
pub struct RttAttachResult {
    pub channels: Vec<RttChannelInfo>,
    pub control_block_address: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FlashProgress {
    pub phase: String,
    pub progress: f64,
    pub message: String,
}

// ── App state ───────────────────────────────────────────────────────────────

pub struct AppState {
    session: Arc<Mutex<Option<Session>>>,
    rtt: Arc<Mutex<Option<RttState>>>,
    rtt_down_buffer: Arc<Mutex<Vec<u8>>>,
    rtt_stop: Arc<AtomicBool>,
}

struct RttState {
    rtt: Rtt,
    up_channel: usize,
    down_channel: usize,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
            rtt: Arc::new(Mutex::new(None)),
            rtt_down_buffer: Arc::new(Mutex::new(Vec::new())),
            rtt_stop: Arc::new(AtomicBool::new(false)),
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
    state.rtt_stop.store(true, Ordering::Relaxed);
    let mut session_guard = state.session.lock();
    let mut rtt_guard = state.rtt.lock();
    state.rtt_down_buffer.lock().clear();
    *rtt_guard = None;
    *session_guard = None;
}

#[tauri::command]
fn flash_firmware(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let mut session_guard = state.session.lock();
    let session = session_guard.as_mut().ok_or("Not connected to a probe")?;

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

    download_file(session, &file_path, format).map_err(|e| format!("Flash failed: {e}"))?;

    Ok(format!("Successfully flashed {path}"))
}

#[tauri::command]
fn rtt_attach(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    up_channel: Option<usize>,
    down_channel: Option<usize>,
    scan_region: Option<String>,
) -> Result<RttAttachResult, String> {
    let (rtt, channels) = {
        let mut session_guard = state.session.lock();
        let session = session_guard.as_mut().ok_or("Not connected to a probe")?;

        let mut core = session
            .core(0)
            .map_err(|e| format!("Failed to get core: {e}"))?;

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

        (rtt, channels)
    }; // session lock released here

    let cb_addr = format!("0x{:x}", rtt.ptr());

    *state.rtt.lock() = Some(RttState {
        rtt,
        up_channel: up_channel.unwrap_or(0),
        down_channel: down_channel.unwrap_or(0),
    });
    state.rtt_down_buffer.lock().clear();

    // Start background RTT thread.
    state.rtt_stop.store(false, Ordering::Relaxed);
    let session_arc = state.session.clone();
    let rtt_arc = state.rtt.clone();
    let down_buffer_arc = state.rtt_down_buffer.clone();
    let stop_flag = state.rtt_stop.clone();

    std::thread::spawn(move || {
        let mut buf = [0u8; 128];
        loop {
            if stop_flag.load(Ordering::Relaxed) {
                break;
            }

            let (data, wrote_down) = {
                let mut session_guard = session_arc.lock();
                let mut rtt_guard = rtt_arc.lock();

                let Some(session) = session_guard.as_mut() else {
                    break;
                };
                let Some(rtt_state) = rtt_guard.as_mut() else {
                    break;
                };

                let Ok(mut core) = session.core(0) else { break };

                let mut output = Vec::new();
                {
                    let Some(up_ch) = rtt_state.rtt.up_channel(rtt_state.up_channel) else {
                        break;
                    };

                    loop {
                        match up_ch.read(&mut core, &mut buf) {
                            Ok(0) => break,
                            Ok(count) => output.extend_from_slice(&buf[..count]),
                            Err(_) => break,
                        }
                    }
                }

                let wrote_down = {
                    let mut down_buffer = down_buffer_arc.lock();
                    if down_buffer.is_empty() {
                        false
                    } else {
                        let Some(down_ch) = rtt_state.rtt.down_channel(rtt_state.down_channel)
                        else {
                            break;
                        };

                        match down_ch.write(&mut core, down_buffer.as_slice()) {
                            Ok(0) => false,
                            Ok(count) => {
                                down_buffer.drain(..count);
                                true
                            }
                            Err(_) => break,
                        }
                    }
                };

                let data = if output.is_empty() {
                    None
                } else {
                    Some(String::from_utf8_lossy(&output).to_string())
                };

                (data, wrote_down)
            }; // locks released here

            let should_sleep = data.is_none() && !wrote_down;

            if let Some(data) = data {
                let _ = app.emit("rtt-data", data);
            }

            if should_sleep {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    });

    Ok(RttAttachResult {
        channels,
        control_block_address: cb_addr,
    })
}

#[tauri::command]
fn rtt_write(state: State<'_, AppState>, data: String) -> Result<usize, String> {
    let session_guard = state.session.lock();
    if session_guard.is_none() {
        return Err("Not connected to a probe".into());
    }

    let mut rtt_guard = state.rtt.lock();
    let rtt_state = rtt_guard.as_mut().ok_or("RTT not attached")?;

    if rtt_state.rtt.down_channel(rtt_state.down_channel).is_none() {
        return Err(format!("Down channel {} not found", rtt_state.down_channel));
    }

    let bytes = data.into_bytes();
    let count = bytes.len();
    state.rtt_down_buffer.lock().extend_from_slice(&bytes);

    Ok(count)
}

#[tauri::command]
fn rtt_detach(state: State<'_, AppState>) {
    state.rtt_stop.store(true, Ordering::Relaxed);
    let mut rtt_guard = state.rtt.lock();
    state.rtt_down_buffer.lock().clear();
    *rtt_guard = None;
}

#[tauri::command]
fn reset_target(state: State<'_, AppState>) -> Result<(), String> {
    let mut session_guard = state.session.lock();
    let session = session_guard.as_mut().ok_or("Not connected to a probe")?;

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
            rtt_write,
            rtt_detach,
            reset_target,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
