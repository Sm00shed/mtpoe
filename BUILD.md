# Building the mtpoe OpenWrt package

`mtpoe` ships as an OpenWrt package built with the `rust/host` toolchain. The
package Makefile fetches the Rust sources from this git repository, so **push
your changes first** — the build clones `HEAD`.

## Prerequisites

- An OpenWrt buildroot checked out and configured for your target
  (RB5009UPr+S+IN → `mvebu/cortexa72`, arch `aarch64_cortex-a72`).
- The Rust host toolchain is pulled automatically (`PKG_BUILD_DEPENDS:=rust/host`).

## 1. Add the package to the buildroot

The Makefile and its runtime files live under `openwrt/` in this repo. Copy them
into your OpenWrt tree so the Makefile and its `files/` directory sit **together**
(the Makefile references `./files/…`):

```
DEST=<openwrt>/package/utils/mtpoe
mkdir -p "$DEST"
cp openwrt/package/utils/mtpoe/Makefile "$DEST/"
cp -r openwrt/files "$DEST/files"
```

## 2. Select and build

```
cd <openwrt>
make defconfig
make menuconfig       # Utilities → <M> mtpoe
# optional, for .apk output:
#   Global build settings → Use APK instead of OPKG
make package/mtpoe/compile V=s -j1
```

## 3. Locate the package

```
find bin -name 'mtpoe_*'
# e.g. bin/packages/aarch64_cortex-a72/base/mtpoe_0.1.0-1_aarch64_cortex-a72.apk
```

## 4. Install on the router

```
apk add ./mtpoe_0.1.0-1_aarch64_cortex-a72.apk     # apk-based OpenWrt
# or, on older opkg-based builds:
opkg install ./mtpoe_0.1.0-1_aarch64_cortex-a72.ipk
```

## What the package installs

- `/usr/sbin/mtpoe` — the binary
- `/etc/init.d/mtpoe` — service; runs `mtpoe apply` at boot (`START=11`)
- `/etc/uci-defaults/99-mtpoe` — creates `/etc/config/mtpoe` on first boot
  (every port `auto`)
- `/etc/config/mtpoe` — default UCI config

Enable and start the service:

```
/etc/init.d/mtpoe enable
/etc/init.d/mtpoe start
```

## Rebuilding after source changes

Push the new commit, then rebuild (the git checkout is cached):

```
make package/mtpoe/{clean,compile} V=s -j1
```
