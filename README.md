# A9 Strip

A **VST3** and **CLAP** audio plugin modeled on the **Pioneer DJ DJM-A9** mixer channel strip, built in Rust with 64-bit internal DSP (`nih-plug` + `egui`).

## Features

- **Exact DJM-A9 Channel Signal Flow:**
  `Input` → `Trim` → `3-Band EQ / Isolator` → `Sound Color FX: Filter` → `Channel Fader` → `Subtle Soft Clipper` → `Output`
- **64-Bit Internal Floating-Point DSP:** All EQ, crossover, and filter math runs in `f64` with per-sample parameter smoothing for zipper-free sweeps.
- **Trim (`-∞ dB` to `+9 dB`):** Unity (`0 dB`) at 12 o'clock.
- **Switchable 3-Band `ISO` / `EQ` (`HI`, `MID`, `LOW`):**
  - **ISO Mode (`-∞ dB` full kill to `+6 dB`):** Phase-aligned 4th-order Linkwitz-Riley (24 dB/oct) 3-band crossover (`280 Hz` / `3.8 kHz`) that sums mathematically flat at `0 dB` and kills cleanly to silence at full counter-clockwise.
  - **EQ Mode (`-26 dB` to `+6 dB`):** Musical Pioneer DJM shelf/bell voicing (`70 Hz` Low Shelf, `1 kHz` Mid Bell, `13 kHz` High Shelf).
  - Click-free crossfade when toggling between `ISO` and `EQ`.
- **Sound Color FX: `FILTER` + `PARAMETER (RES)` + `CTR LOCK`:**
  - **Center-Zero `COLOR / FILTER` Knob:** Bit-transparent bypass at 12 o'clock with smooth center engagement; left sweeps **LPF** (`20 kHz → 25 Hz`), right sweeps **HPF** (`20 Hz → 18 kHz`) using cascaded 4-pole Zero-Delay Feedback (ZDF) State-Variable Filters.
  - **`PARAMETER (RES)` Knob:** Controls filter resonance from smooth Butterworth (`0%`) to aggressive Pioneer club resonance (`100%`) with musical inter-stage saturation and automatic center taper.
  - **`CTR LOCK` Switch:** Signature DJM-A9 Center Lock—when enabled, dragging across 12 o'clock locks at center until you release and drag again.
  - **`FILTER` Enable Button:** Click-free bypass toggle.
- **Vertical Channel Fader (`LEVEL`):** Pioneer P-Lock styled fader (`0.0 dB` full up down to `-∞ dB` mute) with smooth club-mixer taper.
- **Subtle Output Soft Clipper (`SOFT CLIP`):** Bit-transparent below `-1.5 dBFS`, with a smooth `C1`-continuous `tanh` headroom knee above `-1.5 dBFS` and a live `HEADROOM` / `CLIP` indicator.
- **15-Segment LED Level Meter:** Pioneer-style Green / Amber / Red peak meter.

### Controls & Shortcuts
- **Double-click** any knob or the fader to reset to default (`12 o'clock` / `0.0 dB`).
- **`Shift + Drag`** for fine adjustment.
- **Scroll wheel** over any knob or fader for quick tweaks.

## Building from Source

### Prerequisites
- [Rust toolchain](https://rustup.rs/) (stable)
- On Windows: Visual Studio Build Tools (MSVC)

### Run DSP Unit Tests
```bash
cargo test
```

### Build VST3 & CLAP Bundles
```bash
cargo xtask bundle a9_strip --release
```

The compiled plugin bundles will be created at:
- `target/bundled/A9 Strip.vst3`
- `target/bundled/A9 Strip.clap`

## Installing on Windows

Copy `target/bundled/A9 Strip.vst3` into your system VST3 folder:
```powershell
New-Item -ItemType Directory -Force "C:\Program Files\Common Files\VST3" | Out-Null
Copy-Item -Recurse -Force "target\bundled\A9 Strip.vst3" "C:\Program Files\Common Files\VST3\"
```

*(Optional)* For CLAP-compatible DAWs (Bitwig, Reaper, FL Studio, etc.), copy `target/bundled/A9 Strip.clap` to `C:\Program Files\Common Files\CLAP\`.

Rescan plugins in your DAW and load **A9 Strip**.

## License

MIT — see [LICENSE](LICENSE).
