use crate::error::MtpoeError;
use std::process::Command;

/// UCI config file: /etc/config/mtpoe
pub const UCI_CONFIG_FILE: &str = "mtpoe";
/// Section type within /etc/config/mtpoe
pub const DEFAULT_UCI_SECTION: &str = "poe";

/// Reads PoE port values from /etc/config/mtpoe via `uci get` per port.
/// Returns a Vec of (user_port, value) pairs; port is 1-based (chassis label),
/// value is 0=off, 1=force, 2=auto. Only ports explicitly set in UCI are returned.
pub fn load_poe_from_uci(section: &str, ports_num: usize) -> Result<Vec<(usize, u8)>, MtpoeError> {
    let base = format!("{UCI_CONFIG_FILE}.@{section}[0]");
    let mut results = Vec::new();

    for port in 1..=ports_num {
        let output = Command::new("uci")
            .args(["-q", "get", &format!("{base}.port{port}")])
            .output()
            .map_err(|e| MtpoeError::Uci(format!("uci get failed: {e}")))?;

        if !output.status.success() {
            continue; // port not set (or section absent)
        }

        let raw = String::from_utf8_lossy(&output.stdout);
        let val_str = raw.trim();

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

/// True if the `section` section already exists in /etc/config/mtpoe.
fn section_exists(section: &str) -> bool {
    Command::new("uci")
        .args(["-q", "get", &format!("{UCI_CONFIG_FILE}.@{section}[0]")])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Seeds /etc/config/mtpoe with an anonymous `section`, every port set to auto
/// (2). No-op if the section already exists. Returns the number of ports written
/// (0 if it was already present).
pub fn write_default_config(section: &str, ports_num: usize) -> Result<usize, MtpoeError> {
    if section_exists(section) {
        return Ok(0);
    }
    uci(&["add", UCI_CONFIG_FILE, section])?;
    for port in 1..=ports_num {
        uci(&["set", &format!("{UCI_CONFIG_FILE}.@{section}[-1].port{port}=2")])?;
    }
    uci(&["commit", UCI_CONFIG_FILE])?;
    Ok(ports_num)
}

fn uci(args: &[&str]) -> Result<(), MtpoeError> {
    let out = Command::new("uci")
        .args(args)
        .output()
        .map_err(|e| MtpoeError::Uci(format!("uci {}: {e}", args.join(" "))))?;
    if !out.status.success() {
        return Err(MtpoeError::Uci(format!("uci {} failed", args.join(" "))));
    }
    Ok(())
}
