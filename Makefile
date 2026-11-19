CARGO ?= cargo
INSTALL ?= install
PREFIX ?= /usr/local
BINDIR ?= $(PREFIX)/bin
DATADIR ?= $(PREFIX)/share
XSESSIONS_DIR ?= $(DATADIR)/xsessions
WAYLAND_SESSIONS_DIR ?= $(DATADIR)/wayland-sessions
DESTDIR ?=

.PHONY: release install

release:
	$(CARGO) build --release --bins

install:
	test -x target/release/aetherwm-wayland || { echo "Run 'make release' first." >&2; exit 1; }
	test -x target/release/aetherwm-x11 || { echo "Run 'make release' first." >&2; exit 1; }
	$(INSTALL) -D -m 0755 target/release/aetherwm-wayland "$(DESTDIR)$(BINDIR)/aetherwm-wayland"
	$(INSTALL) -D -m 0755 target/release/aetherwm-x11 "$(DESTDIR)$(BINDIR)/aetherwm-x11"
	$(INSTALL) -D -m 0644 packaging/aetherwm-wayland.desktop "$(DESTDIR)$(WAYLAND_SESSIONS_DIR)/aetherwm-wayland.desktop"
	$(INSTALL) -D -m 0644 packaging/aetherwm-x11.desktop "$(DESTDIR)$(XSESSIONS_DIR)/aetherwm-x11.desktop"
	$(INSTALL) -D -m 0755 examples/xinitrc "$(DESTDIR)$(DATADIR)/aetherwm/examples/xinitrc"
