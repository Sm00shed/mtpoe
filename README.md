# mtpoe

PoE port control for MikroTik boards running OpenWrt (ATtiny V2/V3, SAMD20 V4).
Provides per-port **force** — power without 802.3af/at negotiation — which the
standard PSE-PD path does not cover.

## Supported boards

Board, protocol and SPI device are auto-detected from `/tmp/sysinfo/board_name`
(override with `--board` / `--proto` / `--dev`):

- `rb-750p-pbr2` — V2, 4 ports, `/dev/spidev0.2`
- `mikrotik,routerboard-960pgs` — V3, 4 ports, `/dev/spidev0.2`
- `mikrotik,rb5009upr` — V4, 8 ports, `/dev/spidev2.0`

## Installation

OpenWrt package `mtpoe` (requires `kmod-spi-dev`):

```
apk add mtpoe
```

On first boot a uci-defaults script creates `/etc/config/mtpoe` with every port
set to `auto`.

## Commands

```
mtpoe status                # firmware, voltage, temperature, all ports
mtpoe show fw|voltage|temp  # a single reading
mtpoe port                  # all ports and their state
mtpoe port <N>              # a single port (1-based)
mtpoe port <N> <mode>       # set a port
mtpoe apply                 # apply the configuration from UCI
mtpoe probe <cmd> [b1] [b2] # read a raw SPI register (debug)
mtpoe version
```

Global options: `--json` (machine-readable instead of text), `--dev`,
`--uci-key`, `--proto`, `--board`, `--verbose`.

### Port modes

- `off` (0) — port off
- `auto` (2) — standard 802.3af/at; power only for a negotiating PD
- `force` (1) — power unconditionally, without negotiation; for passive or
  non-standard devices (e.g. a Reolink doorbell)

Ports are **1-based** throughout (matching the chassis labels) — in the CLI, UCI
and JSON.

## Configuration (UCI)

`/etc/config/mtpoe`:

```
config poe
    option port1 'auto'
    option port2 'off'
    option port3 'force'
    ...
```

Values: `off` / `auto` / `force` or `0` / `1` / `2`. `mtpoe apply` writes them to
the controller (only changed ports, where the current state can be read back).

## Notes

- **V4 (RB5009):** the admin state of all 8 ports cannot be read back (register
  `0x45` only covers 4 ports), so `poe_config` is `null` on V4 and `apply` writes
  every port. Live per-port status and current work normally.
- **Persistence:** the controller stores port states across reboots and power
  loss — ports are active **before** the OS starts. With `force` this means power
  is applied before OpenWrt is running.
- **`probe`:** sends a correctly framed command (CRC + retry) and shows the raw
  response in hex and decimal, without interpretation. Known brick opcodes
  (flash/reset) are refused without `--force-dangerous`.

## Credits

- [adron-s](https://github.com/adron-s) — original `mtpoe_ctrl` author
- [prudy](https://github.com/prudy) — RB5009UPr research and adaptations
- [Sm00shed](https://github.com/Sm00shed) — maintainer of this Rust rewrite

## License

GPL-2.0
