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

---

## ✨ Key Features

*   **🦀 Pure Rust Architecture:** Built with strict memory safety guarantees, eliminating memory leaks and undefined behavior common in legacy C/C++ window managers.
*   **⚡ Native EWMH & Fastfetch Support:** Out-of-the-box Extended Window Manager Hints atom implementation baked into the core runtime for instant desktop state recognition.
*   **🪶 Featherlight Overhead:** Consuming only 2.2 MiB of RAM, maximizing system cycles for your actual applications.
*   **🛠️ Tiling Purist Control:** Precise master-stack layout management and predictable window mapping.

---

## 🧬 Built to Be Forked & Extended

AetherWM wasn't just built to be used—it was built to be **hacked**. Born out of pure evening boredom and a refusal to accept software bloat, the codebase is intentionally clean, modular, and transparent.

If you want to experiment with custom layout algorithms, tweak event loop dispatching, or build your own bespoke tiling system from a reliable base:
1. Fork the repo: [`azazel21t/AetherWM`](https://github.com/azazel21t/AetherWM/fork)
2. Dive into the core event handlers in `src/`.
3. Build your custom logic on top of a rock-solid, memory-safe Rust foundation.

---

## 🚀 Quick Start & Installation

### Prerequisites
Make sure you have Rust installed along with basic X11 development headers.

### 1. Clone the Repository
```bash
git clone [https://github.com/azazel21t/AetherWM.git](https://github.com/azazel21t/AetherWM.git)
cd AetherWM
