<div align="center">

# AetherWM

**A blazingly fast, uncompromising X11 tiling window manager featuring an infinite spatial canvas, continuous zoom, matrix navigation, and multi-monitor support—engineered completely from scratch in Rust.**  
*Built for minimalists, hackers, and anyone who wants a clean slate to fork, customize, and build upon.*

[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Protocol: X11](https://img.shields.io/badge/Protocol-X11-blue.svg?style=flat-square)](https://www.x.org/)
[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-green.svg?style=flat-square)](https://www.gnu.org/licenses/gpl-3.0)
[![RAM Usage: ~2.2 MiB](https://img.shields.io/badge/RAM-2.2%20MiB-brightgreen.svg?style=flat-square)](https://github.com/azazel21t/AetherWM)

</div>

---

## 🌌 What is AetherWM?

AetherWM breaks away from traditional rigid workspaces by introducing an **infinite spatial canvas and matrix navigation model**. Instead of switching between static numbered tags, you navigate across a boundless 2D coordinate grid (`x, y`), pan seamlessly across open desktop spaces, and zoom in or out of your active window layouts with buttery-smooth scaling.

Written entirely in **Rust** and interfacing directly with X11 and Xrandr, AetherWM guarantees memory safety at compile-time while delivering bare-metal responsiveness and consuming only **~2.2 MiB of RAM**.

---

## 🪶 Performance & Benchmarks

| Metric | Specification |
| :--- | :--- |
| **Active RAM Footprint** | **~2.2 MiB** during multi-window tiling workflows |
| **Primary Test Rig** | HP Laptop 14s-fq0xxx (AMD 3020e, Radeon Vega 3 Integrated Graphics) + External Display |
| **Operating System** | Arch Linux |
| **Display Server** | X11 / Xrandr |
| **Display Support** | **Multi-monitor dynamic clipping & per-monitor tiling** (Mixed resolutions supported) |

---

## ✨ Key Features

*   **🦀 Pure Rust Architecture:** Strict memory safety guarantees, zero memory leaks, and blazing-fast compilation times (`~3 seconds`).
*   **🌐 Infinite Spatial Canvas & Continuous Zoom:** Navigate infinitely across a 2D coordinate matrix. Zoom out for an overview or zoom in instantly with smooth interpolation (`MIN_ZOOM` to `1.0`).
*   **🖥️ Robust Multi-Monitor & Cursor-Aware Spawning:** Automatically detects multi-head setups via Xrandr. New applications spawn directly on the physical monitor where your mouse cursor is resting.
*   **⚡ EWMH Compliance & Fastfetch Ready:** Native Extended Window Manager Hints baked into the core runtime for full state recognition and compositor compatibility (`picom`, `dunst`, `feh`).
*   **🛠️ Flexible `.ars` Configuration & Window Rules:** Customize gaps, border widths, colors, and apply class-based window rules via `config.ars`.

---

## 🖱️ Navigation & Mouse Controls

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

AetherWM reads its configuration from `~/.config/aetherwm/config.ars`. 

### Full Sample Configuration (`config.ars`)
```text
# AetherWM Configuration Reference

# Core layout & styling parameters
border_width = 2
outer_gap = 8
inner_gap = 8
master_ratio = 0.55
default_layout = "tiling" # or "floating"

focused_border_color = "0x7aa2f7"
normal_border_color = "0x2b3346"

# Window Rules (Class-based overrides)
[[window_rule]]
selector_key = "class"
selector_val = "Firefox"
property = "border_width"
value = "2"

[[window_rule]]
selector_key = "class"
selector_val = "Firefox"
property = "border_color"
value = "0x7aa2f7"

# Autostart Applications
autostart = "picom --config ~/.config/picom/picom.conf"
autostart = "feh --bg-scale ~/Pictures/wallpaper_hd.png"
autostart = "dunst"

# Keybindings
[[binding]]
modifiers = ["Mod4"]
key = "Return"
action = "exec kitty"

[[binding]]
modifiers = ["Mod4"]
key = "space"
action = "exec rofi -show drun"

[[binding]]
modifiers = ["Mod4", "Shift"]
key = "q"
action = "killclient"

[[binding]]
modifiers = ["Mod4", "Shift"]
key = "e"
action = "quit"

# Spatial Matrix Navigation
[[binding]]
modifiers = ["Mod4"]
key = "Left"
action = "matrix_left"

[[binding]]
modifiers = ["Mod4"]
key = "Right"
action = "matrix_right"

[[binding]]
modifiers = ["Mod4"]
key = "Up"
action = "matrix_up"

[[binding]]
modifiers = ["Mod4"]
key = "Down"
action = "matrix_down"

# Zoom Controls
[[binding]]
modifiers = ["Mod4"]
key = "equal"
action = "zoom_in"

[[binding]]
modifiers = ["Mod4"]
key = "minus"
action = "zoom_out"

[[binding]]
modifiers = ["Mod4"]
key = "0"
action = "zoom_reset"

[[binding]]
modifiers = ["Mod4"]
key = "o"
action = "toggleoverview"
