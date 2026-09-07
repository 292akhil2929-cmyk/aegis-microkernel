#![no_std]
#![no_main]

use core::arch::{asm, global_asm};
use core::fmt::{self, Write};
use core::panic::PanicInfo;

use aegis_microkernel::capability::{CapabilitySystem, Object, Rights};
use aegis_microkernel::ipc::{IpcOutcome, Message, RendezvousIpc};

global_asm!(include_str!("boot.S"));
global_asm!(include_str!("vectors.S"));

const PL011_BASE: usize = 0x0900_0000;

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

    let mut caps = CapabilitySystem::<4, 8, 16>::new();
    let uart = Object::Mmio {
        base: PL011_BASE as u64,
        pages: 1,
    };
    caps.create_root(0, 0, uart, Rights::ALL).unwrap();
    caps.mint(0, 0, 1, 0, Rights::WRITE).unwrap();
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
    println!("[ready] milestone 1 complete; waiting for interrupts");

    loop {
        unsafe { asm!("wfe") };
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
