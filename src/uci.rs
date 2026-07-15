use crate::error::MtpoeError;
use std::process::Command;

/// UCI config file: /etc/config/mtpoe
pub const UCI_CONFIG_FILE: &str = "mtpoe";
/// Section type within /etc/config/mtpoe
pub const DEFAULT_UCI_SECTION: &str = "poe";

/// Reads PoE port values from /etc/config/mtpoe with a single `uci show`.
/// Returns a Vec of (user_port, value) pairs; port is 1-based (chassis label),
/// value is 0=off, 1=on, 2=auto. Only ports explicitly set in UCI are returned.
pub fn load_poe_from_uci(section: &str, ports_num: usize) -> Result<Vec<(usize, u8)>, MtpoeError> {
    let base = format!("{UCI_CONFIG_FILE}.@{section}[0]");
    let output = Command::new("uci")
        .args(["show", &base])
        .output()
        .map_err(|e| MtpoeError::Uci(format!("uci show failed: {e}")))?;

    if !output.status.success() {
        // Section not present — nothing to apply.
        return Ok(Vec::new());
    }

    // Lines look like: mtpoe.@poe[0].port1='2'
    let text = String::from_utf8_lossy(&output.stdout);
    let mut results = Vec::new();

    for port in 1..=ports_num {
        let prefix = format!("{base}.port{port}=");
        let Some(line) = text.lines().find(|l| l.starts_with(&prefix)) else {
            continue; // port not set
        };
        let val_str = line[prefix.len()..].trim().trim_matches('\'');

        let val: u8 = val_str
            .parse()
            .map_err(|_| MtpoeError::Uci(format!("port{port}: invalid value '{val_str}'")))?;

        if val > 2 {
            return Err(MtpoeError::Uci(format!(
                "port{port}: value {val} out of range (must be 0..2)"
            )));
        }

        results.push((port, val));
    }

    Ok(results)
}
