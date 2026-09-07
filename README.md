# Aegis microkernel

[![kernel-ci](https://github.com/292akhil2929-cmyk/aegis-microkernel/actions/workflows/ci.yml/badge.svg)](https://github.com/292akhil2929-cmyk/aegis-microkernel/actions/workflows/ci.yml)

**Aegis** is a capability-first ARM64 teaching microkernel for QEMU `virt`, built as a semester capstone around phone-style application sandboxing. It is inspired by seL4's small-kernel and explicit-authority principles, but uses an original, deliberately compact **LeaseTree** capability design.

> Current milestone: the kernel launches an EL0 app and console-server context with separate stacks. The app performs a synchronous Call carrying a byte; the console server is the only accepted UART writer and replies to resume the app. Direct app access to PL011 remains hardware-denied. Per-task ASIDs/table roots, queued multi-client endpoints, and timer-driven context switching remain roadmap work.

## What is working

- AArch64 reset entry with EL2-to-EL1 transition and a 16 KiB boot stack.
- Custom linker script at QEMU's `0x4008_0000` direct-kernel load address.
- PL011 serial output at `0x0900_0000`.
- Active 39-bit, 4 KiB-granule stage-1 translation with separate Device and Normal memory attributes.
- 16-entry, 2 KiB-aligned EL1 exception-vector table with ESR/ELR reporting.
- GICv2 CPU/distributor initialization and 10 Hz ARM virtual-timer interrupts.
- IRQ entry preserves all 31 general-purpose registers before Rust dispatch and returns with `eret`.
- Real EL1-to-EL0 transition with isolated executable and stack pages.
- Lower-EL synchronous exception frame handling for `SVC` and data aborts.
- Hardware demonstration that EL0 cannot access the kernel-owned PL011 mapping.
- Two EL0 contexts with separate stacks and a verified cooperative A→B→A switch.
- Synchronous EL0 app→console Call/Receive and console→app Reply flow.
- Console writes restricted to the server execution identity.
- Deterministic physical-frame allocator with reservation, exhaustion, and double-free checks.
- W^X-enforcing AArch64 page-descriptor builder and fixed-capacity address-space mapping policy.
- Fixed-capacity per-task CSpaces with typed kernel objects and explicit rights.
- Rights-attenuating mint and complete derivation-subtree revocation.
- Generation-protected stale capability references.
- Four-register synchronous rendezvous model whose endpoints queue only thread IDs.
- Capability-authorized syscall decoding for endpoint and frame operations.
- Fixed-capacity task contexts and a round-robin scheduler that skips blocked tasks.
- Host unit tests demonstrating allocation safety, W^X, scheduling fairness, denial without authority, attenuation, revocation, typed syscalls, and both rendezvous orders.

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
[mmu] stage-1 identity map: ON
[memory] frame allocator + W^X descriptor: PASS
[caps] console UART capability: GRANTED
[caps] sandbox UART capability: DENIED
[ipc] synchronous rendezvous self-test: PASS
[syscall] typed endpoint authorization: PASS
[sched] round-robin policy self-test: PASS
[timer] enabling GICv2 virtual timer at 10 Hz
[ready] milestone 3 interrupt bring-up; waiting for timer IRQs
[timer] EL1 IRQ delivery: PASS (3 ticks)
[el0] entering sandbox with isolated code and stack pages
[el0] SVC yield round-trip: PASS
[ipc] app Call -> console Receive: PASS
A <- [console-server] capability-authorized write: PASS
[ipc] console Reply -> app resume: PASS
[el0] direct PL011 access: DENIED by stage-1 MMU
[el0] sandbox exception recovery: PASS
[ready] milestone 4 EL0 isolation proof complete
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
