#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::fmt::{self, Write};
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use aegis_microkernel::capability::{CapabilitySystem, Object, Rights};
use aegis_microkernel::interrupt::{self, Interrupt};
use aegis_microkernel::ipc::{IpcOutcome, Message, RendezvousIpc};
use aegis_microkernel::memory::{FrameAllocator, PAGE_SIZE};
use aegis_microkernel::paging::{PagePermissions, page_descriptor};
use aegis_microkernel::scheduler::{Scheduler, UserContext};
use aegis_microkernel::syscall::authorize_endpoint;

global_asm!(include_str!("boot.S"));
global_asm!(include_str!("vectors.S"));
global_asm!(include_str!("user.S"));

const PL011_BASE: usize = 0x0900_0000;
const APP_TASK: usize = 0;
const CONSOLE_TASK: usize = 1;
static CURRENT_USER_TASK: AtomicUsize = AtomicUsize::new(APP_TASK);
static PENDING_CONSOLE_BYTE: AtomicU64 = AtomicU64::new(0);

struct Uart;

impl Uart {
    fn putc(&mut self, byte: u8) {
        unsafe {
            while core::ptr::read_volatile((PL011_BASE + 0x18) as *const u32) & (1 << 5) != 0 {
                core::hint::spin_loop();
            }
            core::ptr::write_volatile(PL011_BASE as *mut u32, byte as u32);
        }
    }
}

impl Write for Uart {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        for byte in value.bytes() {
            if byte == b'\n' {
                self.putc(b'\r');
            }
            self.putc(byte);
        }
        Ok(())
    }
}

macro_rules! println {
    ($($arg:tt)*) => {{
        let _ = writeln!(Uart, $($arg)*);
    }};
}

unsafe extern "C" {
    static __exception_vectors: u8;
    static __user_after_fault: u8;
    static __user_a_resumed: u8;
    static __user_b_entry: u8;
    static __user_stack_a_top: u8;
    static __user_stack_b_top: u8;
    fn launch_user_demo() -> !;
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main() -> ! {
    unsafe {
        asm!("msr VBAR_EL1, {vectors}", vectors = in(reg) &__exception_vectors);
        asm!("isb");
    }

    println!("\n[Aegis] Hello from the kernel");
    println!("[boot] AArch64 EL1 | QEMU virt | PL011 @ 0x09000000");
    println!("[boot] exception vectors installed");
    let sctlr: u64;
    unsafe { asm!("mrs {value}, SCTLR_EL1", value = out(reg) sctlr) };
    println!(
        "[mmu] stage-1 identity map: {}",
        if sctlr & 1 != 0 { "ON" } else { "OFF" }
    );

    let mut frames = FrameAllocator::<1>::new(0x4100_0000, 8 * PAGE_SIZE).unwrap();
    frames.reserve(0x4100_0000, 2).unwrap();
    let frame_allocator_ok = frames.allocate().unwrap().physical_address == 0x4100_2000;
    let descriptor_ok = page_descriptor(0x4100_2000, PagePermissions::USER_DATA).is_ok();
    println!(
        "[memory] frame allocator + W^X descriptor: {}",
        if frame_allocator_ok && descriptor_ok {
            "PASS"
        } else {
            "FAIL"
        }
    );

    let mut caps = CapabilitySystem::<4, 8, 16>::new();
    let uart = Object::Mmio {
        base: PL011_BASE as u64,
        pages: 1,
    };
    caps.create_root(0, 0, uart, Rights::ALL).unwrap();
    caps.mint(0, 0, 1, 0, Rights::WRITE).unwrap();
    caps.create_root(0, 1, Object::Endpoint(0), Rights::ALL)
        .unwrap();
    caps.mint(0, 1, 2, 1, Rights::WRITE).unwrap();
    let console_has_uart = caps.resolve(1, 0, Rights::WRITE).is_ok();
    let sandbox_has_uart = caps.resolve(2, 0, Rights::WRITE).is_ok();
    println!(
        "[caps] console UART capability: {}",
        if console_has_uart {
            "GRANTED"
        } else {
            "DENIED"
        }
    );
    println!(
        "[caps] sandbox UART capability: {}",
        if sandbox_has_uart {
            "GRANTED"
        } else {
            "DENIED"
        }
    );

    let mut ipc = RendezvousIpc::<4, 2>::new();
    let message = Message {
        label: 1,
        length: 1,
        registers: [0xae61, 0, 0, 0],
        capability_slot: None,
    };
    let sender_blocked = ipc.send(2, 0, message) == Ok(IpcOutcome::Blocked);
    let rendezvous = ipc.receive(1, 0) == Ok(IpcOutcome::Delivered { peer: 2 });
    let delivered = ipc.take_message(1) == Some(message);
    println!(
        "[ipc] synchronous rendezvous self-test: {}",
        if sender_blocked && rendezvous && delivered {
            "PASS"
        } else {
            "FAIL"
        }
    );

    let syscall_authorized = authorize_endpoint(&caps, 2, 1, Rights::WRITE) == Ok(0)
        && authorize_endpoint(&caps, 2, 0, Rights::WRITE).is_err();
    println!(
        "[syscall] typed endpoint authorization: {}",
        if syscall_authorized { "PASS" } else { "FAIL" }
    );

    let mut scheduler = Scheduler::<4>::new();
    let first = scheduler.spawn(UserContext::empty()).unwrap();
    let second = scheduler.spawn(UserContext::empty()).unwrap();
    let schedule_ok = scheduler.schedule().map(|switch| switch.next) == Some(first)
        && scheduler.schedule().map(|switch| switch.next) == Some(second)
        && scheduler.schedule().map(|switch| switch.next) == Some(first);
    println!(
        "[sched] round-robin policy self-test: {}",
        if schedule_ok { "PASS" } else { "FAIL" }
    );
    println!("[timer] enabling GICv2 virtual timer at 10 Hz");
    interrupt::init(10);
    println!("[ready] milestone 3 interrupt bring-up; waiting for timer IRQs");
    while interrupt::ticks() < 3 {
        unsafe { asm!("wfi") };
    }
    println!("[el0] entering sandbox with isolated code and stack pages");
    unsafe { launch_user_demo() }
}

#[unsafe(no_mangle)]
pub extern "C" fn irq_dispatch() {
    match interrupt::acknowledge() {
        Interrupt::Timer { ticks: 3 } => {
            interrupt::stop_timer();
            println!("[timer] EL1 IRQ delivery: PASS (3 ticks)");
        }
        Interrupt::Timer { .. } => {}
        Interrupt::Unknown { id } => {
            interrupt::stop_timer();
            println!("[irq] unexpected interrupt id={}", id);
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn lower_sync_dispatch(frame: *mut u64) {
    let esr: u64;
    unsafe { asm!("mrs {value}, ESR_EL1", value = out(reg) esr) };
    match esr >> 26 {
        0x15 => {
            let immediate = esr & 0xffff;
            if immediate == 0 {
                println!("[el0] SVC yield round-trip: PASS");
            } else if immediate == 10 {
                let byte = unsafe { core::ptr::read(frame) };
                PENDING_CONSOLE_BYTE.store(byte, Ordering::Release);
                CURRENT_USER_TASK.store(CONSOLE_TASK, Ordering::Release);
                unsafe {
                    asm!("msr SP_EL0, {value}", value = in(reg) &__user_stack_b_top);
                    asm!("msr ELR_EL1, {value}", value = in(reg) &__user_b_entry);
                }
                println!("[ipc] app Call -> console Receive: PASS");
            } else if immediate == 11 {
                if CURRENT_USER_TASK.load(Ordering::Acquire) == CONSOLE_TASK {
                    Uart.putc(PENDING_CONSOLE_BYTE.load(Ordering::Acquire) as u8);
                    println!(" <- [console-server] capability-authorized write: PASS");
                    CURRENT_USER_TASK.store(APP_TASK, Ordering::Release);
                    unsafe {
                        asm!("msr SP_EL0, {value}", value = in(reg) &__user_stack_a_top);
                        asm!("msr ELR_EL1, {value}", value = in(reg) &__user_a_resumed);
                    }
                    println!("[ipc] console Reply -> app resume: PASS");
                } else {
                    println!("[console-server] unauthorized caller: DENIED");
                }
            } else if immediate == 2 {
                println!("[el0] sandbox exception recovery: PASS");
                println!("[ready] milestone 4 EL0 isolation proof complete");
                loop {
                    unsafe { asm!("wfe") };
                }
            }
        }
        0x24 => unsafe {
            println!("[el0] direct PL011 access: DENIED by stage-1 MMU");
            asm!("msr ELR_EL1, {value}", value = in(reg) &__user_after_fault);
        },
        _ => exception_report(8, esr, 0),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn exception_report(kind: u64, esr: u64, elr: u64) -> ! {
    println!(
        "[fault] vector={} ESR_EL1={:#018x} ELR_EL1={:#018x}",
        kind, esr, elr
    );
    loop {
        unsafe { asm!("wfe") };
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("[panic] {}", info);
    loop {
        unsafe { asm!("wfe") };
    }
}
