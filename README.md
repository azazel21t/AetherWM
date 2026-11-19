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
*Transferring to Wayland Today!*
---

## 🌌 What is AetherWM?

AetherWM is moving from its original X11 window manager to a native Wayland compositor. The Wayland build can run directly on a Linux TTY through libseat, DRM/KMS, GBM, EGL, and libinput. The older X11 window manager remains available as a separate executable.

The compositor includes XDG toplevels, shared-memory buffers, keyboard and pointer input, matrix workspaces, master-stack tiling, overview, configurable keybindings, and a nested development mode.

### Current Wayland build

By default, `cargo run` starts the native compositor and takes over the active local TTY. Run it from a logged-in TTY session; libseat obtains display and input access through logind or seatd. It selects one connected DRM output. To build and run the nested development backend inside an existing desktop, use `cargo run --features wayland-nested -- --nested`.

Both modes create the `wayland-aetherwm` socket and launch configured autostart commands with that socket selected. Layer-shell panels, XWayland, multiple outputs, and output hotplug are not implemented yet. The X11 implementation remains available with `cargo run --bin aetherwm-x11`.

The native build needs the development packages for libseat, libinput, libudev, libdrm, GBM, EGL, and xkbcommon in addition to Rust dependencies.

### Release install

`cargo build --release --bins` leaves both executables in `target/release`. Install them and register the Wayland and X11 sessions:

```sh
sudo make install PREFIX=/usr
```

The Wayland session appears in display managers that scan `/usr/share/wayland-sessions`; the X11 session appears under `/usr/share/xsessions`. For a TTY, log in on a local virtual terminal and run `aetherwm-wayland`. To launch the X11 build with `startx`, put `exec aetherwm-x11` in `~/.xinitrc`; a sample is installed at `/usr/share/aetherwm/examples/xinitrc`.

To stage an install for packaging without writing to system directories, run `DESTDIR=/tmp/aetherwm-package PREFIX=/usr make install` after building.

---

## 🪶 Performance & Benchmarks

| Metric | Specification |
| :--- | :--- |
| **Active RAM Footprint** | To be measured for the Wayland build |
| **Legacy X11 Test Rig** | HP Laptop 14s-fq0xxx (AMD 3020e, Radeon Vega 3 Integrated Graphics) + External Display |
| **Operating System** | Most Linux Distros |
| **Display Server** | Native Wayland compositor; legacy X11 target |
| **Display Support** | One DRM output; multi-output and hotplug support are pending |

---

## ✨ Feature status

The Wayland build currently has matrix workspaces, master-stack tiling, a thumbnail overview, per-window floating and move/resize gestures, keybindings, and nested output VSync. Native DRM page flips are synchronized to the display. The following features belong to the legacy X11 target and have not all been ported:

*   Infinite spatial canvas, continuous zoom, and animated transitions.
*   Xrandr multi-monitor management and cursor-aware window spawning.
*   XRender blur, rounded corners, opacity, and shadows.
*   X11 window rules and EWMH integration.

### Wayland controls

The default bindings are read from `config.ars` or `config.asr`. A config file's `bind` lines replace the built-in bindings.

| Action | Default |
| :--- | :--- |
| Launch Kitty | `Super + Return` |
| Exit compositor | `Super + Shift + Escape` |
| Close focused window | `Super + q` |
| Toggle focused window floating | `Super + Shift + Space` |
| Show overview; click a window to leave it | `Super + p` |
| Change master/stack ratio | `Super + Equal` / `Super + Minus` |
| Reset master/stack ratio | `Super + 0` |
| Switch matrix workspace | `Super + h/j/k/l` |
| Move floating window | `Super + Left-drag` |
| Resize floating window | `Super + Right-drag` |

The Wayland build uses border width and colors, optional corner radius, window opacity, focus-follows-mouse, gaps, `master_ratio`, `default_layout`, bindings, and autostart commands. Rounded corners are currently masked using the desktop background color, so use them with gaps and a solid background. `vsync` controls the nested renderer; native DRM presentation follows the display's page-flip timing. Window rules remain X11-only. Autostart and `exec` commands run with the AetherWM Wayland socket selected and `DISPLAY` removed; X11-only apps need XWayland support, which has not been added yet.

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

The Wayland target reads `config.ars` (or `config.asr` as a fallback). In nested mode, `vsync = true` synchronizes the renderer to the host window's refresh; set it to `false` to disable that synchronization. Native DRM output uses page flips synchronized to the display.

### Sample Configuration (`config.ars`)
```text
# AetherWM Configuration Template

border_width = 2
border_color = 0x1f2335
focused_border_color = 0x7aa2f7
corner_radius = 0
window_opacity = 100
focus_follows_mouse = true
inner_gap = 10
outer_gap = 14
master_ratio = 0.50
default_layout = "master_stack"
vsync = true

bind Mod4 + Return -> exec kitty
bind Mod4 + d -> exec wofi --show drun --normal-window
bind Mod4 + q -> killclient
bind Mod4 + r -> reload_config
bind Mod4 + Shift + space -> togglefloating
bind Mod4 + Shift + Escape -> exit
bind Mod4 + p -> toggleoverview
bind Mod4 + minus -> zoom_out
bind Mod4 + equal -> zoom_in
bind Mod4 + 0 -> zoom_reset
bind Mod4 + h -> matrix_left
bind Mod4 + l -> matrix_right
bind Mod4 + k -> matrix_up
bind Mod4 + j -> matrix_down
```

Legacy X11 `comp.conf`:

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
