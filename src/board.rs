use std::fs;
use crate::spi::PoeProto;
use crate::error::MtpoeError;

pub const BOARD_NAME_FILE: &str = "/tmp/sysinfo/board_name";

#[derive(Debug, Clone)]
pub struct PoeBoard {
    /// Space-separated list of board names from /tmp/sysinfo/board_name
    pub names: &'static [&'static str],
    pub proto: PoeProto,
    pub spidev: &'static str,
    pub ports_num: usize,
    /// Maps logical port index → SPI command offset for port status queries.
    /// cmd = POE_CMD_PORT_STATE_BASE + port_state_map[port_index]
    pub port_state_map: &'static [u8],
}

/// All known PoE boards.
/// port_state_map translates logical port number to ATtiny/SAMD20 command offset.
/// The hardware numbers ports in reverse order relative to the logical numbering.
pub static POE_BOARDS: &[PoeBoard] = &[
    PoeBoard {
        names: &["rb-750p-pbr2"],
        proto: PoeProto::V2,
        spidev: "/dev/spidev0.2",
        ports_num: 4,
        port_state_map: &[0xd, 0xc, 0xb, 0xa],
    },
    PoeBoard {
        names: &["mikrotik,routerboard-960pgs"],
        proto: PoeProto::V3,
        spidev: "/dev/spidev0.2",
        ports_num: 4,
        port_state_map: &[0xd, 0xc, 0xb, 0xa],
    },
    PoeBoard {
        // Only the RB5009UPr+S+IN has PoE; the non-PoE variant is a separate
        // board (mikrotik,rb5009ug) and is intentionally not matched here.
        names: &["mikrotik,rb5009upr"],
        proto: PoeProto::V4,
        spidev: "/dev/spidev2.0",
        ports_num: 8,
        port_state_map: &[0x8, 0x7, 0x6, 0x5, 0x4, 0x3, 0x2, 0x1],
    },
];

/// Detect the board by reading /tmp/sysinfo/board_name and matching against known boards.
pub fn detect_board() -> Result<&'static PoeBoard, MtpoeError> {
    let raw = fs::read_to_string(BOARD_NAME_FILE)
        .map_err(|e| MtpoeError::BoardDetection(format!("cannot read {BOARD_NAME_FILE}: {e}")))?;

    let board_name = raw.trim();

    for board in POE_BOARDS {
        for name in board.names {
            if *name == board_name {
                return Ok(board);
            }
        }
    }

    Err(MtpoeError::BoardDetection(format!(
        "unsupported board: '{board_name}'"
    )))
}
