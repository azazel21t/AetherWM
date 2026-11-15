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
| **Operating System** | Most Linux Distros |
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
# ============================================================================
# AetherWM - Comprehensive Master Configuration Template
# Location: ~/.config/aetherwm/config.ars
# ============================================================================

# ----------------------------------------------------------------------------
# 1. VISUALS & GAPS
# ----------------------------------------------------------------------------

# Border width of managed windows in pixels
border_width = 2

# Unfocused window border color (Hex format)
border_color = 0x1f2335

# Active/focused window border color (Tokyo Night blue accent by default)
focused_border_color = 0x7aa2f7

# Spacing between tiled windows in a cell
inner_gap = 10

# Margin spacing around the outer edges of the screen
outer_gap = 14

# Master-to-stack split ratio (0.50 means a 50/50 split)
master_ratio = 0.50

# Default layout mode: "master_stack" or "floating"
default_layout = "master_stack"


# ----------------------------------------------------------------------------
# 2. AUTOSTART COMMANDS
# ----------------------------------------------------------------------------
# Commands executed in the background when AetherWM boots up.
# Perfect for wallpapers (feh), compositors (picom), or status bars.

autostart = "feh --bg-scale ~/.wallpaper.jpg &"
autostart = "picom --config ~/.config/picom/picom.conf &"


# ----------------------------------------------------------------------------
# 3. KEYBINDINGS TUTORIAL
# ----------------------------------------------------------------------------
# Syntax: bind <Modifiers> + <Key> -> <Action>
# Available Modifiers: Mod4 (Super/Windows key), Mod1 (Alt), Shift, Control/Ctrl
#
# Available Actions:
#   - killclient      : Close the currently focused application window
#   - reload_config   : Hot-reload this configuration file without restarting WM
#   - togglefloating  : Switch the focused window between tiled and floating layouts
#   - toggleoverview  : Zoom out to view the spatial canvas layout
#   - zoom_in         : Scale the continuous canvas zoom in
#   - zoom_out        : Scale the continuous canvas zoom out
#   - zoom_reset      : Reset continuous zoom back to 100% (1.0)
#   - matrix_left     : Pan camera viewport one cell to the left
#   - matrix_right    : Pan camera viewport one cell to the right
#   - matrix_up       : Pan camera viewport one cell up
#   - matrix_down     : Pan camera viewport one cell down
#   - exec <command>  : Launch any shell command or program

# --- Application Launchers ---
bind Mod4 + Return -> exec kitty
bind Mod4 + d      -> exec rofi -show drun

# --- Window & Session Management ---
bind Mod4 + q             -> killclient
bind Mod4 + r             -> reload_config
bind Mod4 + Shift + space -> togglefloating

# --- Infinite Canvas & Spatial Matrix Controls ---
bind Mod4 + p     -> toggleoverview
bind Mod4 + minus -> zoom_out
bind Mod4 + equal -> zoom_in
bind Mod4 + 0     -> zoom_reset

# --- Viewport Navigation ---
bind Mod4 + h -> matrix_left
bind Mod4 + l -> matrix_right
bind Mod4 + k -> matrix_up
bind Mod4 + j -> matrix_down


# ----------------------------------------------------------------------------
# 4. WINDOW RULES TUTORIAL
# ----------------------------------------------------------------------------
# Syntax: rule <selector_key>: "<selector_val>" -> <property> = <value>
# Supported selectors: class (Window class name from XClassHint)
# Supported properties: border_color, border_width
#
# Examples: Give specific apps custom styling or borders automatically.

rule class: "Kitty"   -> border_color = 0x7aa2f7
rule class: "Firefox" -> border_width = 3
