#
# Copyright (C) 2018 adron@yapic.net
#

include $(TOPDIR)/rules.mk

PKG_NAME:=mtpoe
PKG_VERSION:=0.1.0
PKG_RELEASE:=1

PKG_SOURCE_PROTO:=git
PKG_SOURCE_URL:=https://github.com/sm00shed/mtpoe.git
PKG_SOURCE_VERSION:=HEAD
PKG_MIRROR_HASH:=skip

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
  DEPENDS:=+kmod-spi-dev
  TITLE:=Per-port PoE control for MikroTik RB5009 and ATtiny/SAMD20 boards
  URL:=$(PKG_SOURCE_URL)
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
	$(INSTALL_DATA) ./files/mtpoe.defaults $(1)/etc/uci-defaults/99-mtpoe
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/mtpoe.config $(1)/etc/config/mtpoe
endef

$(eval $(call BuildPackage,mtpoe))
