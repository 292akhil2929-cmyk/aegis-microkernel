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

The intended completed system keeps only scheduling, address-space switching, exception/interrupt dispatch, capability lookup, and endpoint rendezvous in EL1. UART, allocation policy, naming, and any toy filesystem live in EL0 servers. The current milestone implements and tests the boot path plus the capability and rendezvous cores; EL0 tasks, MMU, GIC, and timer switching are explicit next milestones.

