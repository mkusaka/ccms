# WGPU-Accelerated Interactive Mode

This branch adds experimental GPU-accelerated rendering support to the interactive mode using `ratatui-wgpu`.

## Features

- **GPU Acceleration**: Leverages WebGPU for hardware-accelerated rendering
- **High Performance**: Capable of 800+ FPS at 1080p (compared to ~20 FPS with terminal rendering)
- **Cross-Platform**: Works on Windows, macOS, and Linux with native window rendering
- **Smooth Animations**: High refresh rate enables fluid UI animations

## Building

```bash
# Build with WGPU support
cargo build --release --features wgpu

# Build without WGPU (standard terminal mode)
cargo build --release
```

## Usage

```bash
# Standard terminal-based interactive mode
ccms -i "search query"

# GPU-accelerated interactive mode (requires --features wgpu)
ccms -i --wgpu "search query"
```

## Performance Comparison

| Mode | Backend | Max FPS | Rendering |
|------|---------|---------|-----------|
| Terminal | Crossterm | ~20 FPS | CPU/Terminal |
| WGPU | WebGPU | 800+ FPS | GPU/Native Window |

## Requirements

- Rust 1.75+
- GPU with WebGPU support
- Window system (X11/Wayland on Linux, macOS, Windows)

## Trade-offs

### Advantages
- Significantly higher performance
- Smooth scrolling and animations
- No terminal limitations
- Future support for custom shaders and effects

### Disadvantages
- Requires GPU and window system
- Cannot run in pure terminal environments (SSH, TTY)
- Larger binary size due to graphics dependencies
- Higher initial startup time

## Implementation Details

The WGPU backend:
1. Creates a native window using `winit`
2. Initializes WebGPU rendering context
3. Uses `ratatui-wgpu` as the rendering backend
4. Maintains compatibility with existing TUI components
5. Translates window events to crossterm-compatible key events

## Future Enhancements

- Custom shader effects (blur, glow, etc.)
- Font selection and scaling
- Hardware-accelerated text shaping
- Web browser support via WASM