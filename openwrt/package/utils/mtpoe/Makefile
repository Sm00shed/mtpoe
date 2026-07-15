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

PKG_BUILD_DEPENDS:=rust/host
RUST_PKG_FEATURES:=

include $(INCLUDE_DIR)/package.mk
include $(INCLUDE_DIR)/rust-package.mk

define Package/mtpoe
  SECTION:=utils
  CATEGORY:=Utilities
  DEPENDS:=+kmod-spi-dev
  TITLE:=Management utility for MikroTik PoE V2/V3/V4
  URL:=$(PKG_SOURCE_URL)
endef

define Package/mtpoe/description
  Management utility for MikroTik PoE V2/V3/V4 controllers (ATtiny, SAMD20).
  Replaces mtpoe-ctrl, written in Rust.
endef

define Package/mtpoe/install
	$(INSTALL_DIR) $(1)/usr/sbin
	$(INSTALL_BIN) $(PKG_INSTALL_DIR)/bin/mtpoe $(1)/usr/sbin/mtpoe
	$(INSTALL_DIR) $(1)/etc/init.d
	$(INSTALL_BIN) ./files/99_mtpoe $(1)/etc/init.d/
	$(INSTALL_DIR) $(1)/etc/uci-defaults
	$(INSTALL_BIN) ./files/99_mtpoe.defaults $(1)/etc/uci-defaults/
	$(INSTALL_DIR) $(1)/etc/config
	$(INSTALL_CONF) ./files/mtpoe $(1)/etc/config/mtpoe
endef

$(eval $(call BuildPackage,mtpoe))
