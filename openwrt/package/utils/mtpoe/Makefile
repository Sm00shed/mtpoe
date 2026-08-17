#
# Copyright (C) 2026 Steffen May
#

include $(TOPDIR)/rules.mk

PKG_NAME:=mtpoe
PKG_VERSION:=0.1.2
PKG_RELEASE:=1

PKG_SOURCE_VERSION:=HEAD
PKG_SOURCE_URL:=https://github.com/sm00shed/mtpoe/archive
PKG_SOURCE:=$(PKG_SOURCE_VERSION).tar.gz
PKG_SOURCE_SUBDIR:=mtpoe-$(PKG_SOURCE_VERSION)
PKG_HASH:=0000000000000000000000000000000000000000000000000000000000000000

PKG_MAINTAINER:=Steffen (Sm00shed)
PKG_LICENSE:=GPL-2.0
PKG_LICENSE_FILES:=LICENSE

PKG_BUILD_DEPENDS:=rust/host
RUST_PKG_FEATURES:=

include $(INCLUDE_DIR)/package.mk
include $(TOPDIR)/feeds/packages/lang/rust/rust-package.mk

define Package/mtpoe
  SECTION:=utils
  CATEGORY:=Utilities
  DEPENDS:=+kmod-spi-dev $(RUST_ARCH_DEPENDS)
  TITLE:=Per-port PoE control for MikroTik RB5009 and ATtiny/SAMD20 boards
  URL:=https://github.com/sm00shed/mtpoe
endef

define Package/mtpoe/description
  Per-port PoE control for MikroTik boards (ATtiny V2/V3, SAMD20 V4).
  Exposes per-port force/passive modes beyond IEEE 802.3af/at, which the
  standard PSE-PD path does not provide. Written in Rust.
endef

define Package/mtpoe/conffiles
/etc/config/mtpoe
endef

define Package/mtpoe/install
	$(INSTALL_DIR) $(1)/usr/sbin
	$(INSTALL_BIN) $(PKG_INSTALL_DIR)/bin/mtpoe $(1)/usr/sbin/mtpoe
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/mtpoe.init $(1)/etc/init.d/mtpoe
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/mtpoe.defaults $(1)/etc/uci-defaults/99-mtpoe
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/mtpoe.config $(1)/etc/config/mtpoe
endef

$(eval $(call BuildPackage,mtpoe))
