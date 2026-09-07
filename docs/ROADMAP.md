# Semester roadmap and acceptance gates

| Weeks | Milestone | Evidence required |
|---|---|---|
| 1–2 | EL2/EL1 entry, PL011, vectors | QEMU transcript contains boot banner; forced `brk` reaches exception report |
| 3–4 | frame allocator and 4 KiB page tables | **In progress:** allocator and early MMU active; remaining per-task table materialization and fine-grained W^X kernel mappings |
| 5–6 | EL0 tasks and timer scheduler | **In progress:** EL0 entry, SVC/data-abort recovery, context model, GICv2 timer IRQs, and register-safe vectors; remaining multiple task roots and scheduler-driven context switch |
| 7–9 | LeaseTree capabilities and IPC syscalls | denial, attenuation, transfer, stale-cap, and subtree-revocation tests |
| 10–11 | root task, console server, sandbox app | app cannot touch PL011; can print only through endpoint; revocation kills access |
| 12–13 | benchmarks and hardening | cycle histograms, p50/p95/p99, context switch baseline, reproducible config |
| 14 | report and demo | 4–8 page paper, architecture figures, tagged release, recorded QEMU demo |

## Definition of done

- No device or frame mapping syscall succeeds without a matching typed capability.
- Endpoint payloads are never queued in kernel heap memory.
- Revoking a root invalidates all descendants and releases associated task-owned objects.
- Benchmark results record QEMU version, host CPU, guest CPU model, build profile, warm-up, sample count, and distribution—not only an average.
- Published seL4 figures are discussed as context, not treated as an apples-to-apples comparison.
