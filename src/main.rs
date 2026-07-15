mod board;
mod error;
mod output;
mod spi;
mod uci;

use std::thread;
use std::time::Duration;

use clap::{Parser, Subcommand};
use serde_json::json;

use board::{detect_board, POE_BOARDS};
use error::MtpoeError;
use output::*;
use spi::{
    PoeProto, SpiDevice, POE_CMD_FW_VER, POE_CMD_INP_VOLT, POE_CMD_ON_OFF, POE_CMD_PORT_STATE_BASE,
    POE_CMD_STATE, POE_CMD_TEMPERAT,
};
use uci::{load_poe_from_uci, DEFAULT_UCI_SECTION};

// ── CLI definition ────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "mtpoe", about = "MikroTik PoE controller utility")]
struct Cli {
    /// SPI device path (auto-detected from board if not set)
    #[arg(long)]
    dev: Option<String>,

    /// UCI section type in /etc/config/mtpoe
    #[arg(long, default_value = DEFAULT_UCI_SECTION)]
    uci_key: String,

    /// PoE protocol version (auto-detected if not set)
    #[arg(long)]
    proto: Option<u8>,

    /// Board index (auto-detected if not set)
    #[arg(long)]
    board: Option<usize>,

    /// Verbose SPI debug output on stderr
    #[arg(long, short)]
    verbose: bool,

    /// Repeat command every N seconds (0 = run once)
    #[arg(long, default_value = "0")]
    period: u64,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show all PoE info (fw, voltage, temp, port status)
    Status,
    /// Show firmware version of PoE controller
    Fw,
    /// Show input voltage
    Voltage,
    /// Show temperature
    Temp,
    /// Show PoE port config and status
    Poe {
        /// Port number (1-based, user-facing)
        port: Option<usize>,
        /// Value: off | on | auto
        value: Option<String>,
    },
    /// Load and apply PoE config from UCI
    Apply,
    /// Show this utility version
    Version,
    /// Send raw hex bytes over SPI (debug)
    RawSend {
        /// Hex bytes e.g. "41 00 00"
        hex: String,
    },
}

// ── Runtime context ───────────────────────────────────────────────────────────

struct Context {
    spi: SpiDevice,
    proto: PoeProto,
    ports_num: usize,
    port_state_map: &'static [u8],
    uci_key: String,
}

// ── PoE value helpers ─────────────────────────────────────────────────────────

fn parse_poe_value(s: &str) -> Result<u8, MtpoeError> {
    match s {
        "off" | "0" => Ok(0),
        "on" | "1" => Ok(1),
        "auto" | "2" => Ok(2),
        _ => Err(MtpoeError::InvalidValue(format!(
            "'{s}' — must be off/on/auto or 0/1/2"
        ))),
    }
}

fn poe_config_str(val: u8) -> String {
    match val {
        0 => "off".into(),
        1 => "on".into(),
        2 => "auto".into(),
        _ => "n/a".into(),
    }
}

fn poe_status_value(raw: u16) -> PortStatusValue {
    match raw {
        0x8001 => PortStatusValue::State("auto".into()),
        0x800A => PortStatusValue::State("short".into()),
        0x800F => PortStatusValue::State("on".into()),
        v if v & 0x8000 != 0 => PortStatusValue::State("off".into()),
        v => PortStatusValue::Current(v as u32),
    }
}

/// Translate a chassis port (1..=ports_num, as labelled and used by CLI/UCI/JSON)
/// to the SPI port argument. The hardware numbers ports in reverse order relative
/// to the chassis labels (mtpoe_ctrl.c:277). Bounds-checked here so the reverse
/// mapping can never underflow.
fn hw_port(ports_num: usize, user_port: usize) -> Result<u8, MtpoeError> {
    if user_port < 1 || user_port > ports_num {
        return Err(MtpoeError::InvalidPort(format!(
            "{user_port} — must be 1..{ports_num}"
        )));
    }
    Ok((ports_num - user_port + 1) as u8)
}

// ── Command implementations ───────────────────────────────────────────────────

fn cmd_fw(ctx: &Context) -> Result<(), MtpoeError> {
    let [major, minor] = ctx.spi.query(POE_CMD_FW_VER, 0, 0)?;
    print_json(&FwVersion {
        fw_version: format!("{major}.{minor:02}"),
    });
    Ok(())
}

/// Read the input voltage in volts, rounded to two decimals.
fn read_voltage(ctx: &Context) -> Result<f32, MtpoeError> {
    let [hi, lo] = ctx.spi.query(POE_CMD_INP_VOLT, 0, 0)?;
    let x = (hi as u32) << 8 | lo as u32;
    let v = match ctx.proto {
        PoeProto::V2 => x as f32 * 35.7 / 1024.0,
        PoeProto::V3 | PoeProto::V4 => x as f32 / 100.0,
    };
    Ok((v * 100.0).round() / 100.0)
}

/// V3/V4 temperature conversion: 12-count block formula (mtpoe_ctrl.c:126-142).
fn temp_v3v4_celsius(x: u32) -> i32 {
    let n = x / 12;
    let o = x - n * 12;
    let mut c = (n * 5) as i32 - 273;
    if o > 9 {
        c += 4;
    } else if o > 6 {
        c += 3;
    } else if o > 4 {
        c += 2;
    } else if o > 2 {
        c += 1;
    }
    c
}

/// Read the controller temperature in degrees Celsius.
fn read_temperature(ctx: &Context) -> Result<i32, MtpoeError> {
    let [hi, lo] = ctx.spi.query(POE_CMD_TEMPERAT, 0, 0)?;
    let x = (hi as u32) << 8 | lo as u32;
    let c = match ctx.proto {
        PoeProto::V2 => x as i32 - 273,
        PoeProto::V3 | PoeProto::V4 => temp_v3v4_celsius(x),
    };
    Ok(c)
}

fn cmd_voltage(ctx: &Context) -> Result<(), MtpoeError> {
    print_json(&Voltage {
        voltage_v: read_voltage(ctx)?,
    });
    Ok(())
}

fn cmd_temperature(ctx: &Context) -> Result<(), MtpoeError> {
    print_json(&Temperature {
        temperature_c: read_temperature(ctx)?,
    });
    Ok(())
}

fn get_poe_config(ctx: &Context) -> Result<Option<Vec<PortConfig>>, MtpoeError> {
    if ctx.proto == PoeProto::V4 {
        // TODO: implement V4 port config (SAMD20 does not use POE_CMD_STATE the same way)
        return Ok(None);
    }

    let [hi, lo] = ctx.spi.query(POE_CMD_STATE, 0, 0)?;
    let mut x = (hi as u32) << 8 | lo as u32;
    let mut configs = vec![
        PortConfig {
            port: 0,
            config: String::new()
        };
        ctx.ports_num
    ];

    for i in 0..ctx.ports_num {
        let val = (x & 0xF) as u8;
        let idx = match ctx.proto {
            PoeProto::V2 => i,
            PoeProto::V3 => ctx.ports_num - i - 1,
            PoeProto::V4 => unreachable!(),
        };
        configs[idx] = PortConfig {
            port: idx + 1,
            config: poe_config_str(val),
        };
        x >>= 4;
    }

    Ok(Some(configs))
}

fn get_poe_status(ctx: &Context) -> Result<Vec<PortStatus>, MtpoeError> {
    let mut statuses = Vec::with_capacity(ctx.ports_num);
    for i in 0..ctx.ports_num {
        let cmd = POE_CMD_PORT_STATE_BASE + ctx.port_state_map[i];
        let [hi, lo] = ctx.spi.query(cmd, 0, 0)?;
        let raw = (hi as u16) << 8 | lo as u16;
        statuses.push(PortStatus {
            port: i + 1,
            status: poe_status_value(raw),
        });
    }
    Ok(statuses)
}

fn cmd_poe_show(ctx: &Context) -> Result<(), MtpoeError> {
    let config = get_poe_config(ctx)?;
    let status = get_poe_status(ctx)?;
    print_json(&json!({
        "poe_config": config,
        "poe_status": status,
    }));
    Ok(())
}

fn cmd_poe_set(ctx: &Context, user_port: usize, val: u8) -> Result<(), MtpoeError> {
    if val > 2 {
        return Err(MtpoeError::InvalidValue("PoE value must be 0..2".into()));
    }

    let internal_port = hw_port(ctx.ports_num, user_port)?;
    let [hi, lo] = ctx.spi.query(POE_CMD_ON_OFF, internal_port, val)?;
    if hi != internal_port || lo != val {
        return Err(MtpoeError::Spi(format!(
            "set_poe response mismatch: got 0x{hi:02x}{lo:02x}, expected 0x{internal_port:02x}{val:02x}"
        )));
    }

    print_json(&SetPoeResult {
        status: "ok".into(),
    });
    Ok(())
}

fn cmd_apply(ctx: &Context) -> Result<(), MtpoeError> {
    let uci_ports = load_poe_from_uci(&ctx.uci_key, ctx.ports_num)?;

    // Get current config to avoid unnecessary writes
    let current_raw = if ctx.proto != PoeProto::V4 {
        let [hi, lo] = ctx.spi.query(POE_CMD_STATE, 0, 0)?;
        Some((hi as u32) << 8 | lo as u32)
    } else {
        None
    };

    let mut processed = 0usize;
    let mut new_config: Vec<PortConfig> = (0..ctx.ports_num)
        .map(|i| PortConfig {
            port: i + 1,
            config: "n/a".into(),
        })
        .collect();

    for (user_port, val) in uci_ports {
        // Check current state to avoid redundant SPI writes
        let current_val = if let Some(raw) = current_raw {
            let shift = match ctx.proto {
                PoeProto::V2 => (user_port - 1) * 4,
                PoeProto::V3 => (ctx.ports_num - user_port) * 4,
                PoeProto::V4 => 0,
            };
            Some(((raw >> shift) & 0xF) as u8)
        } else {
            None
        };

        if current_val != Some(val) {
            let internal_port = hw_port(ctx.ports_num, user_port)?;
            ctx.spi.query(POE_CMD_ON_OFF, internal_port, val)?;
            processed += 1;
        }

        new_config[user_port - 1].config = poe_config_str(val);
    }

    print_json(&LoadUciResult {
        status: "ok".into(),
        processed_ports: processed,
        readback: current_raw.is_some(),
        poe_config: new_config,
    });

    Ok(())
}

fn cmd_status(ctx: &Context) -> Result<(), MtpoeError> {
    let [major, minor] = ctx.spi.query(POE_CMD_FW_VER, 0, 0)?;
    let voltage = read_voltage(ctx)?;
    let temp = read_temperature(ctx)?;
    let poe_config = get_poe_config(ctx)?;
    let poe_status = get_poe_status(ctx)?;

    print_json(&FullStatus {
        fw_version: format!("{major}.{minor:02}"),
        voltage_v: voltage,
        temperature_c: temp,
        poe_config,
        poe_status,
    });

    Ok(())
}

fn cmd_raw_send(ctx: &Context, hex: &str) -> Result<(), MtpoeError> {
    let mut tx_data = Vec::new();
    let mut ptr: &str = hex;
    loop {
        let trimmed = ptr.trim_start();
        if trimmed.is_empty() {
            break;
        }
        let (token, rest) = trimmed.split_at(
            trimmed
                .find(|c: char| c.is_whitespace())
                .unwrap_or(trimmed.len()),
        );
        let byte = u8::from_str_radix(token.trim_start_matches("0x"), 16)
            .map_err(|_| MtpoeError::InvalidValue(format!("invalid hex byte: '{token}'")))?;
        tx_data.push(byte);
        ptr = rest;
    }

    if tx_data.is_empty() {
        return Err(MtpoeError::InvalidValue("no bytes to send".into()));
    }

    let rx_data = ctx.spi.raw_query(&tx_data)?;

    let tx_str = tx_data
        .iter()
        .map(|b| format!("0x{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    let rx_str = rx_data
        .iter()
        .map(|b| format!("0x{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ");

    print_json(&RawSendResult {
        action: "raw_send".into(),
        tx: tx_str,
        rx: rx_str,
    });

    Ok(())
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn run() -> Result<(), MtpoeError> {
    let cli = Cli::parse();

    // Board detection
    let board = if let Some(idx) = cli.board {
        POE_BOARDS
            .get(idx.saturating_sub(1))
            .ok_or_else(|| MtpoeError::BoardDetection(format!("board index {idx} out of range")))?
    } else {
        detect_board()?
    };

    let proto = match cli.proto {
        Some(2) => PoeProto::V2,
        Some(3) => PoeProto::V3,
        Some(4) => PoeProto::V4,
        None => board.proto,
        Some(v) => {
            return Err(MtpoeError::InvalidValue(format!(
                "unknown proto version {v}"
            )))
        }
    };

    let dev_path = cli.dev.as_deref().unwrap_or(board.spidev);

    let spi = SpiDevice::open(dev_path, proto, cli.verbose)?;

    let ctx = Context {
        spi,
        proto,
        ports_num: board.ports_num,
        port_state_map: board.port_state_map,
        uci_key: cli.uci_key,
    };

    // Command loop (period=0 → run once, period>0 → monitoring daemon)
    const MAX_CONSECUTIVE_ERRORS: u32 = 3;
    let mut consecutive_errors: u32 = 0;

    loop {
        let result = match &cli.command {
            Commands::Status => cmd_status(&ctx),
            Commands::Fw => cmd_fw(&ctx),
            Commands::Voltage => cmd_voltage(&ctx),
            Commands::Temp => cmd_temperature(&ctx),
            Commands::Poe { port: None, .. } => cmd_poe_show(&ctx),
            Commands::Poe {
                port: Some(p),
                value: Some(v),
            } => {
                let val = parse_poe_value(v)?;
                cmd_poe_set(&ctx, *p, val)
            }
            Commands::Poe {
                port: Some(_),
                value: None,
            } => Err(MtpoeError::InvalidValue(
                "poe <port> requires a value: off|on|auto".into(),
            )),
            Commands::Apply => cmd_apply(&ctx),
            Commands::Version => {
                print_json(&json!({ "version": env!("CARGO_PKG_VERSION") }));
                Ok(())
            }
            Commands::RawSend { hex } => cmd_raw_send(&ctx, hex),
        };

        match result {
            Ok(_) => {
                consecutive_errors = 0;
            }
            Err(e) => {
                print_error(-1, &e.to_string());
                consecutive_errors += 1;

                let should_exit =
                    e.is_fatal() || cli.period == 0 || consecutive_errors >= MAX_CONSECUTIVE_ERRORS;

                if should_exit {
                    eprintln!(
                        "aborting after {} consecutive error(s): {}",
                        consecutive_errors, e
                    );
                    return Err(e);
                }
            }
        }

        if cli.period == 0 {
            break;
        }
        thread::sleep(Duration::from_secs(cli.period));
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        print_error(-1, &e.to_string());
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hw_port_reverses_and_bounds_check() {
        // Chassis labels map to the reversed hardware numbering.
        assert_eq!(hw_port(8, 1).unwrap(), 8);
        assert_eq!(hw_port(8, 8).unwrap(), 1);
        assert_eq!(hw_port(4, 1).unwrap(), 4);
        assert_eq!(hw_port(4, 4).unwrap(), 1);
        // Out of range is rejected, no underflow.
        assert!(hw_port(8, 0).is_err());
        assert!(hw_port(8, 9).is_err());
    }

    #[test]
    fn parse_poe_value_accepts_names_and_digits() {
        assert_eq!(parse_poe_value("off").unwrap(), 0);
        assert_eq!(parse_poe_value("on").unwrap(), 1);
        assert_eq!(parse_poe_value("auto").unwrap(), 2);
        assert_eq!(parse_poe_value("0").unwrap(), 0);
        assert_eq!(parse_poe_value("2").unwrap(), 2);
        assert!(parse_poe_value("bogus").is_err());
    }

    #[test]
    fn temp_v3v4_matches_block_formula() {
        assert_eq!(temp_v3v4_celsius(0), -273);
        assert_eq!(temp_v3v4_celsius(12), -268); // n=1, o=0
        assert_eq!(temp_v3v4_celsius(744), 37); // n=62, o=0
        assert_eq!(temp_v3v4_celsius(743), 36); // n=61, o=11 -> +4
        assert_eq!(temp_v3v4_celsius(6), -271); // n=0, o=6 -> +2
    }

    #[test]
    fn poe_status_value_decodes_flags_and_current() {
        assert!(matches!(poe_status_value(0x8001), PortStatusValue::State(s) if s == "auto"));
        assert!(matches!(poe_status_value(0x800A), PortStatusValue::State(s) if s == "short"));
        assert!(matches!(poe_status_value(0x800F), PortStatusValue::State(s) if s == "on"));
        assert!(matches!(poe_status_value(0x8000), PortStatusValue::State(s) if s == "off"));
        assert!(matches!(
            poe_status_value(0x0061),
            PortStatusValue::Current(97)
        ));
    }
}
