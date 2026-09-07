# Aegis microkernel

[![kernel-ci](https://github.com/292akhil2929-cmyk/aegis-microkernel/actions/workflows/ci.yml/badge.svg)](https://github.com/292akhil2929-cmyk/aegis-microkernel/actions/workflows/ci.yml)

**Aegis** is a capability-first ARM64 teaching microkernel for QEMU `virt`, built as a semester capstone around phone-style application sandboxing. It is inspired by seL4's small-kernel and explicit-authority principles, but uses an original, deliberately compact **LeaseTree** capability design.

> Current milestone: the kernel boots at EL1, installs a complete AArch64 exception-vector table, writes through the QEMU PL011, and runs the same fixed-capacity capability and synchronous rendezvous cores that are host-tested. MMU, EL0 task launch, GIC/timer scheduling, and user-space servers remain roadmap work; this repository does not claim those phases are complete.

## What is working

- AArch64 reset entry with EL2-to-EL1 transition and a 16 KiB boot stack.
- Custom linker script at QEMU's `0x4008_0000` direct-kernel load address.
- PL011 serial output at `0x0900_0000`.
- 16-entry, 2 KiB-aligned EL1 exception-vector table with ESR/ELR reporting.
- Fixed-capacity per-task CSpaces with typed kernel objects and explicit rights.
- Rights-attenuating mint and complete derivation-subtree revocation.
- Generation-protected stale capability references.
- Four-register synchronous rendezvous model whose endpoints queue only thread IDs.
- Host unit tests demonstrating denial without authority, attenuation, revocation, and both rendezvous orders.

See [the architecture](docs/ARCHITECTURE.md) and [the semester gates](docs/ROADMAP.md).

## Verification

CI performs three distinct checks on every push: five host-side mechanism tests, an optimized ARM64 cross-build, and a real QEMU `virt` boot whose UART transcript must contain the kernel banner, sandbox denial, and rendezvous `PASS` line. This is milestone evidence, not a claim that later roadmap phases are implemented.

## Build

Prerequisites: current stable Rust, the `aarch64-unknown-none-softfloat` target, and QEMU with `qemu-system-aarch64`.

```powershell
rustup target add aarch64-unknown-none-softfloat
./scripts/build.ps1
./scripts/run-qemu.ps1
```

Expected serial output:

```text
[Aegis] Hello from the kernel
[boot] AArch64 EL1 | QEMU virt | PL011 @ 0x09000000
[boot] exception vectors installed
[caps] console UART capability: GRANTED
[caps] sandbox UART capability: DENIED
[ipc] synchronous rendezvous self-test: PASS
[ready] milestone 1 complete; waiting for interrupts
```

Exit QEMU with `Ctrl+A`, then `X`.

## Test the mechanism cores on the host

Because `.cargo/config.toml` selects ARM64 by default, override the target with your installed host triple:

```powershell
cargo test --target x86_64-pc-windows-msvc --lib
```

## Debug with GDB

Run `./scripts/debug-qemu.ps1`; QEMU stops before the first instruction and exposes a GDB server on TCP 1234. In an AArch64-aware GDB:

```gdb
file target/aarch64-unknown-none-softfloat/debug/aegis-kernel
target remote :1234
break kernel_main
continue
```

## Research positioning

Aegis borrows the established ideas that capabilities name authority and that endpoint IPC can rendezvous without kernel-buffered payloads. Its original capstone contribution is the constrained LeaseTree design: single-level, fixed-size per-task CSpaces backed by kernel-owned typed derivation nodes, monotonic rights attenuation, generation-based stale-reference rejection, and whole-subtree revocation optimized for destroying an application sandbox in one policy operation.

The evaluation will report IPC Call round trips and context switches as distributions (p50/p95/p99 cycles), compare fast-path and deliberately naive buffered-message variants on identical builds, and use published seL4 measurements only as clearly caveated context because platforms and configurations differ.

## Primary references

- G. Klein et al., [seL4: Formal Verification of an OS Kernel](https://sel4.systems/Info/Docs/seL4-whitepaper.pdf), SOSP 2009.
- seL4 tutorials: [Capabilities](https://docs.sel4.systems/Tutorials/capabilities.html) and [IPC](https://docs.sel4.systems/Tutorials/ipc).
- J. Liedtke, [On Micro-Kernel Construction](https://dl.acm.org/doi/10.1145/224056.224075), SOSP 1995.
- R. and A. Arpaci-Dusseau, [Operating Systems: Three Easy Pieces](https://pages.cs.wisc.edu/~remzi/OSTEP/).
- Arm, [AArch64 Exception Model](https://developer.arm.com/documentation/102412/latest/).
- QEMU, [`virt` generic virtual platform](https://qemu.readthedocs.io/en/master/system/arm/virt.html).
- Rust, [`aarch64-unknown-none` platform support](https://doc.rust-lang.org/rustc/platform-support/aarch64-unknown-none.html).

## License

Dual-licensed under MIT or Apache-2.0.
