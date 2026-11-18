<div align="center">

# AetherWM

**A lightweight Rust Wayland compositor in development, with a legacy X11 window manager retained during the port.**  
*Built for minimalists, hackers, and anyone who wants a clean slate to fork, customize, and build upon.*

[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Protocol: Wayland](https://img.shields.io/badge/Protocol-Wayland-blue.svg?style=flat-square)](https://wayland.freedesktop.org/)
[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-green.svg?style=flat-square)](https://www.gnu.org/licenses/gpl-3.0)
[![Status: Wayland port](https://img.shields.io/badge/Status-Wayland%20port-orange.svg?style=flat-square)](https://github.com/azazel21t/AetherWM)

</div>

---

## 🌌 What is AetherWM?

AetherWM is moving from its original X11 window manager to a native Wayland compositor. The current Wayland build is a nested compositor for development; a direct DRM/session backend and broader desktop protocol support are still in progress.

The port is built on Smithay and already includes XDG toplevels, shared-memory buffers, keyboard and pointer input, matrix workspaces, master-stack tiling, overview, configurable keybindings, and nested-output VSync.

### Current Wayland build

`cargo run` starts the nested Wayland compositor inside the current desktop session. It creates the `wayland-aetherwm` socket and launches configured autostart commands with that socket selected. The existing X11 implementation is still available with `cargo run --bin aetherwm-x11` while features are moved across.

This is the migration base, not yet a replacement for a full login-session compositor: direct DRM/seat startup, layer-shell panels, and XWayland integration remain to be implemented.

### Release install

`cargo build --release --bins` leaves both executables in `target/release`. Install them and register the X11 session separately:

```sh
sudo make install PREFIX=/usr
```

The X11 session then appears in display managers that scan `/usr/share/xsessions`. For `startx`, put `exec aetherwm-x11` in `~/.xinitrc`; a sample is installed at `/usr/share/aetherwm/examples/xinitrc`. The Wayland executable is installed to `/usr/bin/aetherwm-wayland` for nested testing, but it is not registered under `wayland-sessions` because it cannot start a real login session yet.

To stage an install for packaging without writing to system directories, run `DESTDIR=/tmp/aetherwm-package PREFIX=/usr make install` after building.

---

## 🪶 Performance & Benchmarks

| Metric | Specification |
| :--- | :--- |
| **Active RAM Footprint** | To be measured for the Wayland build |
| **Legacy X11 Test Rig** | HP Laptop 14s-fq0xxx (AMD 3020e, Radeon Vega 3 Integrated Graphics) + External Display |
| **Operating System** | Most Linux Distros |
| **Display Server** | Wayland prototype (nested); legacy X11 target |
| **Display Support** | One nested output in the current Wayland build; multi-output support is pending |

---

## ✨ Feature status

The Wayland build currently has matrix workspaces, master-stack tiling, a thumbnail overview, per-window floating and move/resize gestures, keybindings, and nested output VSync. The following features belong to the legacy X11 target and have not all been ported:

*   Infinite spatial canvas, continuous zoom, and animated transitions.
*   Xrandr multi-monitor management and cursor-aware window spawning.
*   XRender blur, rounded corners, opacity, and shadows.
*   X11 window rules and EWMH integration.

### Wayland controls

The default bindings are read from `config.ars` or `config.asr`. A config file's `bind` lines replace the built-in bindings.

| Action | Default |
| :--- | :--- |
| Launch Kitty | `Super + Return` |
| Close focused window | `Super + q` |
| Toggle focused window floating | `Super + Shift + Space` |
| Show overview; click a window to leave it | `Super + p` |
| Change master/stack ratio | `Super + Equal` / `Super + Minus` |
| Reset master/stack ratio | `Super + 0` |
| Switch matrix workspace | `Super + h/j/k/l` |
| Move floating window | `Super + Left-drag` |
| Resize floating window | `Super + Right-drag` |

The Wayland build currently uses gaps, `master_ratio`, `default_layout`, bindings, autostart commands, and `vsync`. Border settings and window rules are still X11-only. Autostart and `exec` commands run with the nested Wayland socket selected and `DISPLAY` removed; X11-only apps need XWayland support, which has not been added yet.

---

## 🖱️ Legacy X11 navigation & mouse controls

AetherWM blends keyboard-driven tiling efficiency with fluid mouse interaction:

| Action | Control / Gesture |
| :--- | :--- |
| **Pan Canvas** | Left-drag on empty desktop **OR** `Super + Middle-drag` anywhere |
| **Zoom In / Out** | `Super + Scroll Up / Down` |
| **Move Window** | `Super + Left-drag` on a window *(dragging across workspace boundaries drops it into a new matrix cell)* |
| **Resize Window** | `Super + Right-drag` on a window |
| **Focus Window** | Left-click any window |

---

## ⚙️ Configuration & Customization Reference

The main settings live in `~/.config/aetherwm/config.ars`. The legacy X11 target also reads `~/.config/aetherwm/comp.conf` for its XRender effects; the Wayland target does not use that X11 compositor config.

The Wayland target reads `config.ars` (or `config.asr` as a fallback). Its `vsync = true` setting synchronizes the nested renderer to the host window's refresh; set it to `false` to disable that synchronization.

### Sample Configuration (`config.ars`)
```text
outer_gap = 14
inner_gap = 10
master_ratio = 0.50
default_layout = "master_stack"
vsync = true

# Autostart native Wayland clients; XWayland is not available yet.
# autostart = "kitty"
bind Mod4 + Return -> exec kitty
bind Mod4 + r -> reload_config
bind Mod4 + h -> matrix_left
bind Mod4 + l -> matrix_right
bind Mod4 + equal -> zoom_in
bind Mod4 + minus -> zoom_out
```

The legacy X11 target's `comp.conf` controls its compositor effects:

```text
enabled = true
corner_radius = 12
blur = true
opacity = 92
shadow = true
shadow_opacity = 28
animations = true
animation_duration_ms = 140
```
