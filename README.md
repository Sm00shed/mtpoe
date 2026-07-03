# openwrt-mtpoe-feed

OpenWrt package feed for `mtpoe` — PoE port control for MikroTik devices running OpenWrt.

## Usage

Add to `feeds.conf.default`: src-git mtpoe https://github.com/Sm00shed/openwrt-mtpoe-feed.git

Then:

```bash
./scripts/feeds update mtpoe
./scripts/feeds install mtpoe
```

## Target

Currently tested on MikroTik RB5009UPr+S+IN.

## Credits

- [adron-s](https://github.com/adron-s) — Original `mtpoe_ctrl` author
- [prudy](https://github.com/prudy) — RB5009UPr research and adaptations
- [Sm00shed](https://github.com/Sm00shed) — Feed maintainer

## License

GPL-2.0
