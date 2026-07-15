use crate::error::MtpoeError;
use crc::{Crc, CRC_8_MAXIM_DOW};
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

// SPI configuration
const SPI_MODE: u8 = 0;
const SPI_BITS: u8 = 8;
const SPI_SPEED_HZ: u32 = 2_200_000;
const INTERBYTE_DELAY_USEC: u16 = 150;
/// Per-word delay of the spidev transfer (u8 field in the kernel ABI).
/// Same 150 µs as INTERBYTE_DELAY_USEC, but a dedicated u8 constant so the
/// value cannot be silently truncated by an `as u8` cast.
const WORD_DELAY_USEC: u8 = 150;
const FRAME_LEN: usize = 10;
const MAX_RETRY: usize = 10;

// PoE SPI commands
pub const POE_CMD_FW_VER: u8 = 0x41;
pub const POE_CMD_INP_VOLT: u8 = 0x42;
pub const POE_CMD_TEMPERAT: u8 = 0x43;
pub const POE_CMD_ON_OFF: u8 = 0x44;
pub const POE_CMD_STATE: u8 = 0x45;
pub const POE_CMD_PORT_STATE_BASE: u8 = 0x50;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoeProto {
    V2,
    V3,
    V4,
}

/// ioctl structures for spidev — Linux kernel ABI
#[repr(C)]
struct SpiIocTransfer {
    tx_buf: u64,
    rx_buf: u64,
    len: u32,
    speed_hz: u32,
    delay_usecs: u16,
    bits_per_word: u8,
    cs_change: u8,
    tx_nbits: u8,
    rx_nbits: u8,
    word_delay_usecs: u8,
    pad: u8,
}

nix::ioctl_read!(spi_ioc_rd_mode, b'k', 1, u8);
nix::ioctl_write_ptr!(spi_ioc_wr_mode, b'k', 1, u8);
nix::ioctl_read!(spi_ioc_rd_bits_per_word, b'k', 3, u8);
nix::ioctl_write_ptr!(spi_ioc_wr_bits_per_word, b'k', 3, u8);
nix::ioctl_read!(spi_ioc_rd_max_speed_hz, b'k', 4, u32);
nix::ioctl_write_ptr!(spi_ioc_wr_max_speed_hz, b'k', 4, u32);

// SPI_IOC_MESSAGE(1) — send 1 transfer.
// _IOW('k', 0, struct spi_ioc_transfer): dir=write (0x40), size=0x20 (32 =
// size_of::<SpiIocTransfer>()), type='k' (0x6b), nr=0. The explicit u64 buffer
// fields keep the struct at 32 bytes on both 32- and 64-bit targets.
const SPI_IOC_MESSAGE_1: u64 = 0x4020_6b00;

/// Dallas/Maxim CRC-8 used by the ATtiny/SAMD20 protocol
fn dallas_crc8(data: &[u8]) -> u8 {
    let crc = Crc::<u8>::new(&CRC_8_MAXIM_DOW);
    crc.checksum(data)
}

/// Opened SPI device with initialized parameters
pub struct SpiDevice {
    file: File,
    proto: PoeProto,
    verbose: bool,
}

impl SpiDevice {
    pub fn open(path: &str, proto: PoeProto, verbose: bool) -> Result<Self, MtpoeError> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| MtpoeError::Spi(format!("cannot open {path}: {e}")))?;

        let fd = file.as_raw_fd();
        let dev = Self {
            file,
            proto,
            verbose,
        };

        // Initialize SPI parameters
        unsafe {
            let mut mode = SPI_MODE;
            spi_ioc_wr_mode(fd, &mode)
                .map_err(|e| MtpoeError::Spi(format!("set spi mode: {e}")))?;
            spi_ioc_rd_mode(fd, &mut mode)
                .map_err(|e| MtpoeError::Spi(format!("get spi mode: {e}")))?;

            let mut bits = SPI_BITS;
            spi_ioc_wr_bits_per_word(fd, &bits)
                .map_err(|e| MtpoeError::Spi(format!("set bits per word: {e}")))?;
            spi_ioc_rd_bits_per_word(fd, &mut bits)
                .map_err(|e| MtpoeError::Spi(format!("get bits per word: {e}")))?;

            let mut speed = SPI_SPEED_HZ;
            spi_ioc_wr_max_speed_hz(fd, &speed)
                .map_err(|e| MtpoeError::Spi(format!("set speed: {e}")))?;
            spi_ioc_rd_max_speed_hz(fd, &mut speed)
                .map_err(|e| MtpoeError::Spi(format!("get speed: {e}")))?;

            if dev.verbose {
                eprintln!("spi mode: {mode}");
                eprintln!("bits per word: {bits}");
                eprintln!("max speed: {speed} Hz ({} KHz)", speed / 1000);
            }
        }

        Ok(dev)
    }

    /// Send a command and return the 2 payload bytes from the response.
    /// Retries up to MAX_RETRY times on CRC/framing errors.
    pub fn query(&self, cmd: u8, arg1: u8, arg2: u8) -> Result<[u8; 2], MtpoeError> {
        let fd = self.file.as_raw_fd();

        for attempt in 0..=MAX_RETRY {
            if attempt > 0 {
                let delay = INTERBYTE_DELAY_USEC as u64 * attempt as u64;
                std::thread::sleep(std::time::Duration::from_micros(delay));
            }

            // Build TX frame: [cmd, arg1, arg2, crc, 0, 0, 0, 0, 0, 0]
            let mut tx = [0u8; FRAME_LEN];
            tx[0] = cmd;
            tx[1] = arg1;
            tx[2] = arg2;
            tx[3] = dallas_crc8(&tx[0..3]);

            let mut rx = [0u8; FRAME_LEN];

            let tr = SpiIocTransfer {
                tx_buf: tx.as_ptr() as u64,
                rx_buf: rx.as_mut_ptr() as u64,
                len: FRAME_LEN as u32,
                speed_hz: SPI_SPEED_HZ,
                delay_usecs: INTERBYTE_DELAY_USEC,
                bits_per_word: SPI_BITS,
                cs_change: 0,
                tx_nbits: 0,
                rx_nbits: 0,
                word_delay_usecs: WORD_DELAY_USEC,
                pad: 0,
            };

            if self.verbose {
                eprint!("tx: ");
                for b in &tx {
                    eprint!("{b:02X} ");
                }
                eprintln!();
            }

            let ret = unsafe { libc::ioctl(fd, SPI_IOC_MESSAGE_1, &tr as *const _) };

            if self.verbose {
                eprint!("rx: ");
                for b in &rx {
                    eprint!("{b:02X} ");
                }
                eprintln!();
            }

            if ret < 1 {
                if attempt < MAX_RETRY {
                    continue;
                }
                return Err(MtpoeError::Spi("ioctl failed".into()));
            }

            if ret as usize != FRAME_LEN {
                if attempt < MAX_RETRY {
                    continue;
                }
                return Err(MtpoeError::Spi(format!(
                    "expected {FRAME_LEN} bytes, got {ret}"
                )));
            }

            // Response layout: [pad x4] [tx_crc_echo] [cmd_echo] [data0] [data1] [rx_crc] [rx_crc]
            let resp = &rx[4..];
            let tx_crc_expected = match self.proto {
                PoeProto::V3 | PoeProto::V4 => 0xFF,
                _ => tx[3],
            };

            if resp[0] != tx_crc_expected {
                if attempt < MAX_RETRY {
                    continue;
                }
                return Err(MtpoeError::SpiCrc(format!(
                    "tx crc echo: got 0x{:02x}, expected 0x{:02x}",
                    resp[0], tx_crc_expected
                )));
            }

            if resp[1] != cmd {
                if attempt < MAX_RETRY {
                    continue;
                }
                return Err(MtpoeError::SpiCmd(format!(
                    "cmd echo: got 0x{:02x}, expected 0x{:02x}",
                    resp[1], cmd
                )));
            }

            // rx_crc covers [cmd, data0, data1], repeated twice
            let rx_crc = dallas_crc8(&resp[1..4]);
            if rx_crc != resp[4] || rx_crc != resp[5] {
                if attempt < MAX_RETRY {
                    continue;
                }
                return Err(MtpoeError::SpiCrc(format!(
                    "rx crc: got 0x{:02x}/0x{:02x}, expected 0x{:02x}",
                    resp[4], resp[5], rx_crc
                )));
            }

            return Ok([resp[2], resp[3]]);
        }

        Err(MtpoeError::Spi("max retries exceeded".into()))
    }

    /// Send raw bytes and return the response. Used for debugging/raw_send.
    pub fn raw_query(&self, tx_data: &[u8]) -> Result<Vec<u8>, MtpoeError> {
        let fd = self.file.as_raw_fd();
        let mut rx = vec![0u8; tx_data.len()];

        let tr = SpiIocTransfer {
            tx_buf: tx_data.as_ptr() as u64,
            rx_buf: rx.as_mut_ptr() as u64,
            len: tx_data.len() as u32,
            speed_hz: SPI_SPEED_HZ,
            delay_usecs: INTERBYTE_DELAY_USEC,
            bits_per_word: SPI_BITS,
            cs_change: 0,
            tx_nbits: 0,
            rx_nbits: 0,
            word_delay_usecs: WORD_DELAY_USEC,
            pad: 0,
        };

        if self.verbose {
            eprint!("tx: ");
            for b in tx_data {
                eprint!("{b:02X} ");
            }
            eprintln!();
        }

        let ret = unsafe { libc::ioctl(fd, SPI_IOC_MESSAGE_1, &tr as *const _) };

        if self.verbose {
            eprint!("rx: ");
            for b in &rx {
                eprint!("{b:02X} ");
            }
            eprintln!();
        }

        if ret < 1 {
            return Err(MtpoeError::Spi("raw ioctl failed".into()));
        }

        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::dallas_crc8;

    #[test]
    fn crc8_matches_hardware_reference() {
        // Hardware-verified reference values for the Dallas/Maxim CRC-8.
        assert_eq!(dallas_crc8(&[0x47, 0xAA, 0x00]), 0x42);
        assert_eq!(dallas_crc8(&[0x48, 0x01, 0x23]), 0x11);
    }
}
