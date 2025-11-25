# ani2hyprtui

<div align="center">

[![CI](https://github.com/Sevilze/ani2hyprtui/actions/workflows/ci.yml/badge.svg)](https://github.com/Sevilze/ani2hyprtui/actions/workflows/ci.yml) [![Release](https://github.com/Sevilze/ani2hyprtui/actions/workflows/release.yml/badge.svg)](https://github.com/Sevilze/ani2hyprtui/actions/workflows/release.yml)

</div>

**ani2hyprtui** is a robust, terminal-based user interface (TUI) tool designed to convert Windows cursor themes (animated `.ani` and static `.cur` files) into the Hyprcursor format. Built entirely in **Rust**, it provides a seamless and efficient conversion pipeline without relying on external legacy tools or dependencies.

## Features

* **Native Conversion**: Reads `.ani` and `.cur` files directly and produces Hyprcursor-compatible output.
* **Interactive TUI**: A rich terminal interface for managing the conversion process.
* **Visual Hotspot Editor**: Fine-tune cursor hotspots with a visual preview and real-time animation.
* **Mapping System**: Intelligent remapping of Windows cursor names to X11/Hyprland standards.
* **Batch Processing**: Convert entire directories of cursors in one go.

## Installation

To build `ani2hyprtui`, you need a working Rust toolchain (Cargo).

1. **Clone the repository**:

    ```bash
    git clone https://github.com/Sevilze/ani2hyprtui.git
    cd ani2hyprtui
    ```

2. **Build the project**:

    ```bash
    cargo build --release
    ```

3. **Run the binary**:

    ```bash
    ./target/release/ani2hyprtui
    ```

### Arch Linux (AUR)

```bash
paru -S ani2hyprtui-bin
```

Or using yay:

```bash
yay -S ani2hyprtui-bin
```

### Debian / Ubuntu

Download the `.deb` file from the [Releases](https://github.com/Sevilze/ani2hyprtui/releases) page and install it:

```bash
sudo dpkg -i ani2hypr_*.deb
sudo apt-get install -f  # Fix dependencies if needed
```

### Fedora / Red Hat

Download the `.rpm` file from the [Releases](https://github.com/Sevilze/ani2hyprtui/releases) page and install it:

```bash
sudo rpm -i ani2hypr-*.rpm
```

### Nix Installation

```bash
nix run github:Sevilze/ani2hyprtui/latest
nix profile install github:Sevilze/ani2hyprtui/latest
```

Or enter a development shell (uses main branch by default):

```bash
nix develop github:Sevilze/ani2hyprtui
```

## Usage Guide

The application is divided into several key components, each handling a specific aspect of the workflow. Navigation is primarily keyboard-driven, following standard TUI conventions (Vim-like keys are supported).

---

### 1. File Browser

The File Browser allows you to navigate your filesystem to select input directories (containing `.ani` files) and output directories.

**Controls:**

* `j` / `Down Arrow`: Move selection down.
* `k` / `Up Arrow`: Move selection up.
* `Enter`: Enter the selected directory.
* `l`: Select the current directory as the target for the active operation.

---

### 2. Pipeline Runner

The Runner is the control center for the conversion process. It displays the current status of the conversion pipeline.

**Status Indicators:**

* **Idle**: No operation currently running.
* **Running**: Conversion in progress.
* **Completed**: Successfully processed files.
* **Failed**: An error occurred during processing.

**Usage:**

* Set your **Input Directory** using the File Browser.
* Set your **Output Directory** (where the Hyprcursor theme will be generated).

---

### 3. Mapping Editor

Windows and Linux (X11/Hyprland) use different naming conventions for cursors (e.g., "arrow" vs "left_ptr"). The Mapping Editor allows you to define how these names translate.

**Controls:**

* `j` / `Down Arrow`: Select next mapping.
* `k` / `Up Arrow`: Select previous mapping.
* `Enter` / `e`: Edit the selected mapping.
  * Opens a popup list of available source files found in the input directory.
  * Select a file to assign it to the current X11 name.
* `s`: Save the current mapping configuration.

---

### 4. Hotspot Editor

The Hotspot Editor is a powerful tool for visually adjusting the "hotspot" (the active pixel) of a cursor. Incorrect hotspots make cursors feel "off" or inaccurate.

**Features:**

* **Visual Preview**: See the cursor image and the hotspot location in real-time.
* **Animation Support**: Preview animated cursors to ensure the hotspot remains valid across all frames.
* **Variant Support**: Handle multiple sizes (variants) of the same cursor.

**Controls:**

* **Navigation**:
  * `j` / `k`: Select next/previous cursor in the list.
  * `[` / `]`: Cycle through different size variants (e.g., 32x32, 48x48).
* **Animation**:
  * `Space`: Play/Pause animation.
  * `.` (Period): Step forward one frame.
  * `,` (Comma): Step backward one frame.
* **Editing**:
  * `Arrow Keys`: Move the hotspot pixel by pixel.
  * `s`: Save modified hotspots.

---

### 5. Logs

The Logs component provides real-time feedback on the application's operations. It is essential for troubleshooting and verifying that actions (like saving mappings or converting files) have completed successfully.

**Controls:**

* `j` / `k`: Scroll the log view up and down.
* `PageUp` / `PageDown`: Scroll by pages.

## Troubleshooting

**"Missing source file" in Mapping Editor**
If a mapping shows as Red, it means the file expected for that cursor name (e.g., "arrow.cur") is not in the input directory. You can:

1. Rename a file in your input directory to match.
2. Use the Mapping Editor to select a different available file.
3. Ignore it if you have a "Normal" fallback set up.

**Permission Denied**
Ensure you have write permissions for the output directory. The tool needs to create folders and write binary files.

## Credits

This project includes code and logic adapted from the following open-source projects:

* **[xcur2png](https://github.com/eworm-de/xcur2png)**: For the logic regarding XCursor parsing and PNG extraction.
* **[win2xcur](https://github.com/quantum5/win2xcur)**: For understanding the conversion process from Windows cursor formats.
* **[hyprcursor](https://github.com/hyprwm/hyprcursor)**: For the official Hyprcursor format specification.
