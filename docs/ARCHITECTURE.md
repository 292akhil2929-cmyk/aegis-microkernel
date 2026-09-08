# Aegis architecture

## Protection model

The kernel recognizes task IDs, CSpace slot numbers, and kernel-owned capability nodes. User code never receives a kernel pointer. A syscall is authorized only after the kernel resolves the caller's slot and verifies the requested right and object type.

```text
root task CSpace                 console CSpace          sandbox CSpace
slot 0: UART MMIO [RWGC] --mint--> slot 0: UART [W]      slot 0: empty
slot 1: endpoint [RWG]  --mint--> slot 1: endpoint [R]   slot 1: endpoint [W]
            |
            +-- derivation node -- child -- grandchild
                    revoke(root) invalidates all descendants
```

This is the **LeaseTree** model: authority is a kernel-owned derivation tree, while each task only sees local slot numbers. Minting requires `GRANT`, rights can only be attenuated, and revocation walks and invalidates the complete subtree. Generation counters prevent stale node references from becoming valid when storage is reused.

## IPC contract

An endpoint stores queues of blocked thread IDs, never message payloads. A sender's four message registers remain in its TCB while it is blocked. When a receiver arrives, the kernel copies the registers directly into the receiver's TCB and makes both runnable. The reverse order works identically. This preserves rendezvous semantics and bounds the fast path.

```text
client                   endpoint                 server
Call(ep, MR0..MR3)  ->   sender waits
                          sender ID only    <-    Receive(ep)
                    direct register transfer
client blocks on one-shot reply authority   ->   Reply()
```

Capability transfer is a mint operation performed during rendezvous: the sender names a source slot, the receiver names an empty destination slot, and the kernel applies rights attenuation before installing the derived node.

## Boot flow

```text
QEMU `-kernel` -> `_start` at 0x4008_0000
               -> mask interrupts and select boot stack
               -> if EL2, configure AArch64 EL1 and `eret`
               -> clear BSS, install VBAR_EL1
               -> initialize PL011 console
               -> create root capabilities
               -> idle with WFE
```

## Kernel / user boundary

The intended completed system keeps only scheduling, address-space switching, exception/interrupt dispatch, capability lookup, and endpoint rendezvous in EL1. UART, allocation policy, naming, and any toy filesystem live in EL0 servers. The current milestone implements the early hardware MMU map and tests the physical allocator, mapping policy, task scheduler, typed syscall authorization, capability system, and rendezvous core. Per-task hardware tables, EL0 entry, GIC, and timer-driven context switching are explicit next milestones.

## Memory bring-up

The early boot map deliberately uses a minimal three-level, 39-bit regime with two 1 GiB level-1 blocks:

| Virtual/physical range | Attribute | Purpose |
|---|---|---|
| `0x0000_0000–0x3fff_ffff` | Device-nGnRnE, XN | QEMU `virt` MMIO including PL011 |
| `0x4000_0000–0x7fff_ffff` | Normal WBWA | QEMU RAM and the directly loaded kernel |

`MAIR_EL1`, `TCR_EL1`, and `TTBR0_EL1` are programmed before `SCTLR_EL1.M/C/I` are enabled. This identity map is a safe bring-up map, not the final isolation layout. The next stage replaces the broad executable RAM block with fine-grained W^X mappings and assigns each EL0 task its own table root and ASID.

## Interrupt path

QEMU `virt` is pinned to GICv2 for a stable teaching target. Boot resets distributor enable, pending, priority, and trigger state; enables virtual-timer PPI 27; configures the CPU interface; then programs `CNTV_CVAL_EL0`. The current-EL-with-SPx IRQ vector saves `x0–x30`, calls the Rust dispatcher, restores the complete frame, and executes `eret`. The timer is disabled after the third CI-observed tick so the proof transcript is deterministic.

## EL0 isolation proof

The first 2 MiB of RAM is split into 4 KiB L3 pages. Kernel pages remain privileged; the demo task receives one read/execute code page and one read/write, execute-never stack page. `eret` enters EL0t with `SP_EL0` and `ELR_EL1` initialized. An `SVC` returns through the lower-AArch64 synchronous vector. A subsequent store to the privileged PL011 device mapping produces an EL0 data abort; EL1 records the denial, redirects `ELR_EL1` to the recovery label, and safely returns to the sandbox.

Milestone 6 maps a second user stack and uses the two contexts as an app and console server. The app's Call supplies a byte in saved `x0`, blocks conceptually while EL1 redirects execution to the server, and resumes only after the server's Reply. UART access is accepted only while the console-server identity is current. The complete exception frame remains preserved on the kernel stack. Endpoint wait queues already exist in the tested mechanism core; wiring them to multiple hardware contexts is the next step.

Milestone 8 removes the temporary authorization flag. A single-core runtime owns the actual `CapabilitySystem`: root holds UART and endpoint roots, console holds an attenuated UART-write capability, and the app holds an attenuated endpoint-write capability. Every live operation resolves its typed slot. Revoking root's endpoint slot invalidates both the root and derived app nodes; the subsequent EL0 retry therefore fails with an empty CSpace slot.
