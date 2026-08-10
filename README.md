# NexusOS

![Banner](https://via.placeholder.com/800x200.png?text=NexusOS)

## About

NexusOS is an experimental bare-metal operating system written in Rust. It includes a custom x86_64 kernel (`nexus_kernel`) with memory management, interrupt handling, VGA text output, a minimal VFS, system calls, and basic task scheduling. A companion native desktop environment (`nexus_desktop`) provides the user-space interface.

## Features

- Bare-metal x86_64 kernel (`no_std`)
- Custom allocator, GDT, and interrupt handling
- VGA text buffer and keyboard input
- Minimal VFS, PCI, and network stubs
- System call interface and process/task management
- Native desktop environment built with Rust/Cargo

## Project Structure

```
nexus_kernel/    Bare-metal kernel, build config, and QEMU scripts
nexus_desktop/   Desktop environment source
```

## Installation & Build

### Kernel
Requires the Rust toolchain defined in `rust-toolchain.toml`.

```bash
cd nexus_kernel
cargo build
```

Run with the provided `run.bat` or configure your hypervisor using `.cargo/config.toml`.

### Desktop

```bash
cd nexus_desktop
cargo build
cargo run
```

## Usage

The kernel targets `x86_64-nexus_os`. Launch via `run.bat` or QEMU. The desktop runs as a standard Rust binary.

## Status

Early-stage research. Not intended for production use.
