<div align="center">

# AetherWM

**A blazingly fast, uncompromising X11 tiling window manager engineered completely from scratch in Rust.**  
*Built for minimalists, hackers, and anyone who wants a clean slate to fork, customize, and build upon.*

[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Protocol: X11](https://img.shields.io/badge/Protocol-X11-blue.svg?style=flat-square)](https://www.x.org/)
[![License: GPLv3](https://img.shields.io/badge/License-GPLv3-green.svg?style=flat-square)](https://www.gnu.org/licenses/gpl-3.0)
[![RAM Usage: 2.2 MiB](https://img.shields.io/badge/RAM-2.2%20MiB-brightgreen.svg?style=flat-square)](https://github.com/azazel21t/AetherWM)

</div>

---

## 🌌 What is AetherWM?

AetherWM is a lightweight tiling window manager designed to strip away decades of legacy software bloat, configuration complexity, and heavy desktop environment daemons. Written entirely in **Rust**, it guarantees memory safety at compile-time while delivering bare-metal responsiveness.

Whether you're running it on constrained hardware or using it as a clean foundation for your own custom window manager project, AetherWM gets out of your way and gives you total control.

---

## 🪶 Performance & Benchmarks

Unlike bloated desktop stacks that consume gigabytes of background resources, AetherWM operates with near-zero overhead.

| Metric | Specification |
| :--- | :--- |
| **Active RAM Footprint** | **~2.2 MiB** during multi-window tiling workflows |
| **Primary Test Rig** | HP Laptop 14s-fq0xxx (AMD 3020e, Radeon Vega 3 Integrated Graphics) |
| **Operating System** | Arch Linux |
| **Display Server** | X11 |
| **Display Support** | **Single-display setups only** *(Multi-monitor configurations have NOT been tested yet!)* |

---

## ✨ Key Features

*   **🦀 Pure Rust Architecture:** Built with strict memory safety guarantees, eliminating memory leaks and undefined behavior common in legacy C/C++ window managers.
*   **⚡ Native EWMH & Fastfetch Support:** Out-of-the-box Extended Window Manager Hints atom implementation baked into the core runtime for instant desktop state recognition.
*   **🪶 Featherlight Overhead:** Consuming only 2.2 MiB of RAM, maximizing system cycles for your actual applications.
*   **🛠️ Tiling Purist Control:** Precise master-stack layout management and predictable window mapping.

---

## ⚙️ Configuration & Customization Reference

AetherWM reads its configuration from `~/.config/aetherwm/config.asr`. Below is every customizable parameter you can configure:

### Available Configuration Parameters

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `mod_key` | String | `"Mod4"` | Primary modifier key (`"Mod4"` = Super/Windows key, `"Mod1"` = Alt) |
| `workspaces` | Integer | `4` | Total number of active virtual workspaces |
| `border_width` | Integer | `2` | Window border thickness in pixels |
| `gaps` | Integer | `8` | Spacing between tiled windows in pixels |
| `focused_border_color` | Hex String | `"#58a6ff"` | Border color for the currently active/focused window |
| `normal_border_color` | Hex String | `"#2b3346"` | Border color for inactive background windows |
| `bind` | Array/String | *See template* | Custom keybinding rules (`"Modifier + Key", "action"`) |

### Supported Keybinding Actions
*   `exec <command>` — Launches any shell command or application (e.g., `exec kitty`, `exec rofi -show drun`)
*   `close` — Closes the currently focused window (`Mod4 + Shift + Q`)
*   `quit` — Exits the AetherWM session (`Mod4 + Shift + E`)
*   `workspace <num>` — Switches to the specified virtual workspace (`workspace 1` through `4`)

### Full Sample Configuration (`config.asr`)
```text
# AetherWM Full Configuration Reference
mod_key = "Mod4"

workspaces = 4
border_width = 2
gaps = 8

focused_border_color = "#58a6ff"
normal_border_color = "#2b3346"

# Keybindings
bind = ["Mod4 + Return", "exec kitty"]
bind = ["Mod4 + Space", "exec rofi -show drun"]
bind = ["Mod4 + Shift + Q", "close"]
bind = ["Mod4 + Shift + E", "quit"]

bind = ["Mod4 + 1", "workspace 1"]
bind = ["Mod4 + 2", "workspace 2"]
bind = ["Mod4 + 3", "workspace 3"]
bind = ["Mod4 + 4", "workspace 4"]
