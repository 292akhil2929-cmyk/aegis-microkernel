//! QEMU `virt` GICv2 and ARM generic virtual timer bring-up.

use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

const GICD_BASE: usize = 0x0800_0000;
const GICC_BASE: usize = 0x0801_0000;
const VIRTUAL_TIMER_PPI: u32 = 27;

static TIMER_PERIOD: AtomicU64 = AtomicU64::new(0);
static TIMER_TICKS: AtomicU64 = AtomicU64::new(0);

pub fn init(periodic_hz: u64) {
    unsafe {
        // Reset the QEMU GICv2 state before admitting PPI 27.
        write32(GICD_BASE, 0);
        write32(GICD_BASE + 0x180, u32::MAX);
        write32(GICD_BASE + 0x280, u32::MAX);
        for register in 0..16 {
            write32(GICD_BASE + 0x400 + register * 4, u32::MAX);
        }
        let priority_address = GICD_BASE + 0x400 + (VIRTUAL_TIMER_PPI as usize / 4) * 4;
        let priority_shift = (VIRTUAL_TIMER_PPI % 4) * 8;
        let priority = read32(priority_address) & !(0xff << priority_shift);
        write32(priority_address, priority);
        let config_address = GICD_BASE + 0xc00 + (VIRTUAL_TIMER_PPI as usize / 16) * 4;
        let config_shift = (VIRTUAL_TIMER_PPI % 16) * 2;
        let config = (read32(config_address) & !(0b11 << config_shift)) | (0b10 << config_shift);
        write32(config_address, config);
        write32(GICD_BASE + 0x100, 1 << VIRTUAL_TIMER_PPI);
        write32(GICD_BASE, 1);

        write32(GICC_BASE, 0);
        write32(GICC_BASE + 0x004, 0xff);
        write32(GICC_BASE + 0x008, 0);
        loop {
            let pending = read32(GICC_BASE + 0x00c);
            if pending & 0x3ff == 0x3ff {
                break;
            }
            write32(GICC_BASE + 0x010, pending);
        }
        write32(GICC_BASE, 1);

        let frequency: u64;
        asm!("mrs {value}, CNTFRQ_EL0", value = out(reg) frequency);
        let period = core::cmp::max(1, frequency / periodic_hz);
        TIMER_PERIOD.store(period, Ordering::Relaxed);
        program_next(period);
        asm!("msr CNTV_CTL_EL0, {value}", value = in(reg) 1u64);
        asm!("isb");
        asm!("msr daifclr, #2");
    }
}

pub fn acknowledge() -> Interrupt {
    let raw = unsafe { read32(GICC_BASE + 0x00c) };
    let id = raw & 0x3ff;
    let interrupt = if id == VIRTUAL_TIMER_PPI {
        let ticks = TIMER_TICKS.fetch_add(1, Ordering::Relaxed) + 1;
        let period = TIMER_PERIOD.load(Ordering::Relaxed);
        unsafe {
            asm!("msr CNTV_CTL_EL0, {value}", value = in(reg) 0u64);
            write32(GICD_BASE + 0x280, 1 << VIRTUAL_TIMER_PPI);
            program_next(period);
            asm!("msr CNTV_CTL_EL0, {value}", value = in(reg) 1u64);
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
        asm!("msr CNTV_CTL_EL0, {value}", value = in(reg) 0u64);
        asm!("msr daifset, #2");
    }
}

pub fn ticks() -> u64 {
    TIMER_TICKS.load(Ordering::Relaxed)
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

unsafe fn program_next(period: u64) {
    let counter: u64;
    unsafe {
        asm!("mrs {value}, CNTVCT_EL0", value = out(reg) counter);
        asm!("msr CNTV_CVAL_EL0, {value}", value = in(reg) counter + period);
    }
}
