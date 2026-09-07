//! QEMU `virt` GICv2 and ARM generic physical timer bring-up.

use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

const GICD_BASE: usize = 0x0800_0000;
const GICC_BASE: usize = 0x0801_0000;
const PHYSICAL_TIMER_PPI: u32 = 30;

static TIMER_PERIOD: AtomicU64 = AtomicU64::new(0);
static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

pub fn init(periodic_hz: u64) {
    unsafe {
        // Mark the non-secure physical timer PPI as Group 1 and enable it.
        let group = read32(GICD_BASE + 0x080);
        write32(GICD_BASE + 0x080, group | (1 << PHYSICAL_TIMER_PPI));
        write32(GICD_BASE + 0x100, 1 << PHYSICAL_TIMER_PPI);
        write32(GICD_BASE, 1);

        write32(GICC_BASE + 0x004, 0xff);
        write32(GICC_BASE, 1);

        let frequency: u64;
        asm!("mrs {value}, CNTFRQ_EL0", value = out(reg) frequency);
        let period = core::cmp::max(1, frequency / periodic_hz);
        TIMER_PERIOD.store(period, Ordering::Relaxed);
        asm!("msr CNTP_TVAL_EL0, {value}", value = in(reg) period);
        asm!("msr CNTP_CTL_EL0, {value}", value = in(reg) 1u64);
        asm!("isb");
        asm!("msr daifclr, #2");
    }
}

pub fn acknowledge() -> Interrupt {
    let raw = unsafe { read32(GICC_BASE + 0x00c) };
    let id = raw & 0x3ff;
    let interrupt = if id == PHYSICAL_TIMER_PPI {
        let ticks = TIMER_TICKS.fetch_add(1, Ordering::Relaxed) + 1;
        let period = TIMER_PERIOD.load(Ordering::Relaxed);
        unsafe {
            asm!("msr CNTP_TVAL_EL0, {value}", value = in(reg) period);
        }
        Interrupt::Timer { ticks }
    } else {
        Interrupt::Unknown { id }
    };
    unsafe { write32(GICC_BASE + 0x010, raw) };
    interrupt
}

pub fn stop_timer() {
    unsafe {
        asm!("msr CNTP_CTL_EL0, {value}", value = in(reg) 0u64);
        asm!("msr daifset, #2");
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Interrupt {
    Timer { ticks: u64 },
    Unknown { id: u32 },
}

unsafe fn read32(address: usize) -> u32 {
    unsafe { core::ptr::read_volatile(address as *const u32) }
}

unsafe fn write32(address: usize, value: u32) {
    unsafe { core::ptr::write_volatile(address as *mut u32, value) }
}
