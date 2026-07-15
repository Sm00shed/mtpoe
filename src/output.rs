use serde::Serialize;
use std::io::IsTerminal;

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

/// A single port's config (None on V4) and live status.
#[derive(Serialize)]
pub struct PortDetail {
    pub port: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
    pub status: PortStatusValue,
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum PortStatusValue {
    Current(u32),  // active port — current in mA
    State(String), // off / auto / short / on (force-on no load)
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

/// Result of a `probe`: the framed request and raw response, plus the 16-bit
/// data word as hex and decimal. No unit interpretation — the register may be
/// unknown.
#[derive(Serialize)]
pub struct ProbeResult {
    pub action: String,
    pub cmd: String,
    pub b1: u8,
    pub b2: u8,
    pub tx: String,
    pub rx: String,
    pub data_hex: String,
    pub data_dec: u16,
}

/// All ports' config (None on V4) and live status, for `port` (list).
#[derive(Serialize)]
pub struct PortList {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poe_config: Option<Vec<PortConfig>>,
    pub poe_status: Vec<PortStatus>,
}

#[derive(Serialize)]
pub struct ToolVersion {
    pub version: String,
}

// ── Human-readable rendering ──────────────────────────────────────────────────

/// Types that can render themselves as a compact human-readable line/block.
pub trait Human {
    fn human(&self) -> String;
}

/// Print `value` as JSON (machine mode) or as human-readable text (default).
pub fn emit<T: Serialize + Human>(value: &T, json: bool) {
    if json {
        print_json(value);
    } else {
        println!("{}", value.human());
    }
}

fn status_word(v: &PortStatusValue) -> String {
    match v {
        PortStatusValue::State(s) => s.clone(),
        PortStatusValue::Current(ma) => format!("{ma} mA"),
    }
}

/// Render the indented per-port lines shared by `status` and `port`.
fn render_ports(config: &Option<Vec<PortConfig>>, status: &[PortStatus]) -> String {
    let mut out = String::new();
    for s in status {
        let cfg = config
            .as_ref()
            .and_then(|list| list.iter().find(|c| c.port == s.port));
        match cfg {
            Some(c) => out.push_str(&format!(
                "  {}  {:<6} {}\n",
                s.port,
                c.config,
                status_word(&s.status)
            )),
            None => out.push_str(&format!("  {}  {}\n", s.port, status_word(&s.status))),
        }
    }
    out.truncate(out.trim_end().len());
    out
}

impl Human for FwVersion {
    fn human(&self) -> String {
        format!("firmware: {}", self.fw_version)
    }
}

impl Human for Voltage {
    fn human(&self) -> String {
        format!("voltage: {:.2} V", self.voltage_v)
    }
}

impl Human for Temperature {
    fn human(&self) -> String {
        format!("temperature: {} °C", self.temperature_c)
    }
}

impl Human for SetPoeResult {
    fn human(&self) -> String {
        self.status.clone()
    }
}

impl Human for PortDetail {
    fn human(&self) -> String {
        let st = status_word(&self.status);
        match &self.config {
            Some(c) => format!("port {}: {} ({})", self.port, st, c),
            None => format!("port {}: {}", self.port, st),
        }
    }
}

impl Human for PortList {
    fn human(&self) -> String {
        render_ports(&self.poe_config, &self.poe_status)
    }
}

impl Human for FullStatus {
    fn human(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("{:<13}{}\n", "firmware:", self.fw_version));
        s.push_str(&format!("{:<13}{:.2} V\n", "voltage:", self.voltage_v));
        s.push_str(&format!(
            "{:<13}{} °C\n",
            "temperature:", self.temperature_c
        ));
        s.push_str("ports:\n");
        s.push_str(&render_ports(&self.poe_config, &self.poe_status));
        s
    }
}

impl Human for LoadUciResult {
    fn human(&self) -> String {
        format!(
            "applied {} port(s) (readback: {})",
            self.processed_ports,
            if self.readback { "yes" } else { "no" }
        )
    }
}

impl Human for ProbeResult {
    fn human(&self) -> String {
        format!(
            "cmd {} b1 {} b2 {}\ntx {}\nrx {}\ndata {} ({})",
            self.cmd, self.b1, self.b2, self.tx, self.rx, self.data_hex, self.data_dec
        )
    }
}

impl Human for ToolVersion {
    fn human(&self) -> String {
        format!("mtpoe {}", self.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_single_values() {
        assert_eq!(
            FwVersion {
                fw_version: "65.21".into()
            }
            .human(),
            "firmware: 65.21"
        );
        assert_eq!(
            Temperature { temperature_c: 31 }.human(),
            "temperature: 31 °C"
        );
        assert_eq!(Voltage { voltage_v: 50.1 }.human(), "voltage: 50.10 V");
    }

    #[test]
    fn human_port_list_with_config() {
        let list = PortList {
            poe_config: Some(vec![
                PortConfig {
                    port: 1,
                    config: "auto".into(),
                },
                PortConfig {
                    port: 2,
                    config: "off".into(),
                },
            ]),
            poe_status: vec![
                PortStatus {
                    port: 1,
                    status: PortStatusValue::Current(97),
                },
                PortStatus {
                    port: 2,
                    status: PortStatusValue::State("off".into()),
                },
            ],
        };
        assert_eq!(list.human(), "  1  auto   97 mA\n  2  off    off");
    }

    #[test]
    fn human_port_list_without_config() {
        let list = PortList {
            poe_config: None,
            poe_status: vec![PortStatus {
                port: 1,
                status: PortStatusValue::State("auto".into()),
            }],
        };
        assert_eq!(list.human(), "  1  auto");
    }
}
