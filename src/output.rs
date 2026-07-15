use std::io::IsTerminal;
use serde::Serialize;

/// Print a serializable value as JSON.
/// Pretty-prints if stdout is a terminal, compact if piped.
pub fn print_json<T: Serialize>(value: &T) {
    let json = if std::io::stdout().is_terminal() {
        serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    } else {
        serde_json::to_string(value).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    };
    println!("{json}");
}

/// Print an error as JSON to stdout (for backward compatibility with callers parsing JSON output).
pub fn print_error(code: i32, description: &str) {
    let val = serde_json::json!({
        "error": code,
        "error_description": description
    });
    println!("{}", serde_json::to_string_pretty(&val).unwrap());
}

// ── Output data structures ────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct FwVersion {
    pub fw_version: String,
}

#[derive(Serialize)]
pub struct Voltage {
    pub voltage_v: f32,
}

#[derive(Serialize)]
pub struct Temperature {
    pub temperature_c: i32,
}

#[derive(Serialize, Clone)]
pub struct PortConfig {
    pub port: usize,
    pub config: String,
}

#[derive(Serialize)]
pub struct PortStatus {
    pub port: usize,
    pub status: PortStatusValue,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum PortStatusValue {
    Current(u32),       // active port — current in mA
    State(String),      // off / auto / short / on (force-on no load)
}

#[derive(Serialize)]
pub struct FullStatus {
    pub fw_version: String,
    pub voltage_v: f32,
    pub temperature_c: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poe_config: Option<Vec<PortConfig>>,
    pub poe_status: Vec<PortStatus>,
}

#[derive(Serialize)]
pub struct SetPoeResult {
    pub status: String,
}

#[derive(Serialize)]
pub struct LoadUciResult {
    pub status: String,
    pub processed_ports: usize,
    /// Whether the current on-chip state could be read back before writing.
    /// false on V4 (0x45 is not usable for 8 ports), where processed_ports
    /// therefore counts every configured port rather than only the changed ones.
    pub readback: bool,
    pub poe_config: Vec<PortConfig>,
}

#[derive(Serialize)]
pub struct RawSendResult {
    pub action: String,
    pub tx: String,
    pub rx: String,
}
