# 🌌 Singularity

**The Ultimate Unified Quick-Settings Applet for the COSMIC™ Desktop Environment**

Singularity is a comprehensive, all-in-one control center designed exclusively for COSMIC. Instead of cluttering your panel with separate applets for audio, network, and power, Singularity brings all your essential system controls into one beautifully unified, blazing-fast interface.

Built with Rust and `libcosmic`, Singularity is lightweight, fully integrated with your system's native services, and designed to look and feel like a seamless part of the COSMIC ecosystem.

---

## ✨ Features

- **📶 Network Management (NetworkManager):** Instantly toggle Wi-Fi, connect to saved networks, or activate your VPN. Click on any network to seamlessly drop into your system settings.
- **🎧 Advanced Audio Controls (PipeWire):** Adjust volume, quickly mute/unmute, and switch between audio output and input devices on the fly. Dynamically syncs with WirePlumber and standard PipeWire setups.
- **🎵 Media Playback (MPRIS):** View currently playing media, track titles, and control playback (Play, Pause, Next, Previous) for any MPRIS-compatible media player (Spotify, Firefox, VLC, etc.).
- **🦷 Bluetooth Integration (BlueZ):** Toggle Bluetooth state and view available/connected devices at a glance.
- **🔋 Smart Power Management:** 
  - Switch between Balanced, Performance, and Power-Saver profiles.
  - Universal power actions (Lock, Suspend, Power Off, Log Out) built directly on DBus (`org.freedesktop.login1`), ensuring 100% compatibility across both `systemd` and alternative init systems using `elogind`.
- **🖥️ Hardware Controls:** Display brightness and keyboard backlight sliders. 
- **🧠 Graceful UI Degradation:** Built for desktops and laptops alike. Singularity dynamically adapts to your hardware—automatically hiding battery status or backlight sliders if the hardware isn't present, keeping your interface clean.

## 🚀 Getting Started

### Prerequisites

To build Singularity, you will need standard Linux development tools and the COSMIC platform libraries:

- `rust` and `cargo`
- `just` (command runner)
- Standard Linux desktop services running in the background:
  - NetworkManager
  - UPower
  - BlueZ
  - PipeWire
  - `systemd-logind` or `elogind`

### Building and Installing

Singularity provides a `justfile` that makes building and installing extremely simple.

1. **Clone the repository:**
   ```bash
   git clone https://github.com/sreevikramr/singularity.git
   cd singularity
   ```

2. **Build the optimized release version:**
   ```bash
   just build-release
   ```

3. **Install to your system:**
   *(This copies the binary and registers the desktop entry so COSMIC can find it)*
   ```bash
   sudo just install
   ```

### Adding to your COSMIC Panel

Once installed, enabling Singularity is just a few clicks away:

1. Right-click anywhere on your COSMIC panel and select **Panel Settings**.
2. Navigate to the **Applets** section.
3. Click **+ Add Applet**.
4. Locate **Singularity** in the list and click it to add it to your panel.
5. *(Optional)* Remove the default individual network, audio, and battery applets to enjoy your new unified setup!

## 🛠️ Development & Contributing

Singularity is an actively developing project. We welcome pull requests, bug reports, and feature requests!

### Useful Commands

- `just run` — Builds and runs the applet directly in a window for easy UI testing.
- `just check` — Runs `cargo clippy` to ensure your code matches the project's linting standards.
- `just clean` — Removes built artifacts.
- `sudo just uninstall` — Completely removes the applet from your system.

## 📝 Documentation

For more information on building COSMIC applets, refer to the [libcosmic API documentation](https://pop-os.github.io/libcosmic/cosmic/) and the [libcosmic book](https://pop-os.github.io/libcosmic-book/).

## ⚖️ License

This project is licensed under the [GPL-3.0 License](LICENSE).