use std::process::Command;
use crate::error::MtpoeError;

/// UCI config file: /etc/config/mtpoe
pub const UCI_CONFIG_FILE: &str = "mtpoe";
/// Section type within /etc/config/mtpoe
pub const DEFAULT_UCI_SECTION: &str = "poe";

/// Reads PoE port values from /etc/config/mtpoe.
/// Returns a Vec of (port_index, value) pairs where value is 0=off, 1=on, 2=auto.
/// Only returns ports that are explicitly set in UCI.
pub fn load_poe_from_uci(
    section: &str,
    ports_num: usize,
) -> Result<Vec<(usize, u8)>, MtpoeError> {
    let mut results = Vec::new();

    for port in 0..ports_num {
        let key = format!("{UCI_CONFIG_FILE}.@{section}[0].port{port}");
        let output = Command::new("uci")
            .args(["get", &key])
            .output()
            .map_err(|e| MtpoeError::Uci(format!("uci get failed: {e}")))?;

        if !output.status.success() {
            // Port not set in UCI — skip
            continue;
        }

        let val_str = String::from_utf8_lossy(&output.stdout);
        let val_str = val_str.trim();

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
