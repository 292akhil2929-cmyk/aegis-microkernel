//! Live single-core kernel state tying CSpaces to the EL0 demo contexts.

use core::cell::UnsafeCell;

use crate::capability::{CapError, CapabilitySystem, Object, Rights};
use crate::syscall::{SyscallError, authorize_endpoint, authorize_mmio};

pub const ROOT_TASK: usize = 0;
pub const CONSOLE_TASK: usize = 1;
pub const APP_TASK: usize = 2;
const UART_SLOT: usize = 0;
const ENDPOINT_SLOT: usize = 1;

type LiveCaps = CapabilitySystem<4, 8, 16>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    Capability(CapError),
    Syscall(SyscallError),
    WrongTask,
}

impl From<CapError> for RuntimeError {
    fn from(value: CapError) -> Self {
        Self::Capability(value)
    }
}

impl From<SyscallError> for RuntimeError {
    fn from(value: SyscallError) -> Self {
        Self::Syscall(value)
    }
}

struct Runtime {
    capabilities: LiveCaps,
    current_task: usize,
    pending_byte: u8,
}

impl Runtime {
    const fn empty() -> Self {
        Self {
            capabilities: LiveCaps::new(),
            current_task: APP_TASK,
            pending_byte: 0,
        }
    }

    fn bootstrap(&mut self) -> Result<(), RuntimeError> {
        self.capabilities.create_root(
            ROOT_TASK,
            UART_SLOT,
            Object::Mmio {
                base: 0x0900_0000,
                pages: 1,
            },
            Rights::ALL,
        )?;
        self.capabilities
            .mint(ROOT_TASK, UART_SLOT, CONSOLE_TASK, UART_SLOT, Rights::WRITE)?;
        self.capabilities.create_root(
            ROOT_TASK,
            ENDPOINT_SLOT,
            Object::Endpoint(0),
            Rights::ALL,
        )?;
        self.capabilities.mint(
            ROOT_TASK,
            ENDPOINT_SLOT,
            APP_TASK,
            ENDPOINT_SLOT,
            Rights::WRITE,
        )?;
        Ok(())
    }

    fn app_call(&mut self, byte: u8) -> Result<(), RuntimeError> {
        if self.current_task != APP_TASK {
            return Err(RuntimeError::WrongTask);
        }
        authorize_endpoint(&self.capabilities, APP_TASK, ENDPOINT_SLOT, Rights::WRITE)?;
        self.pending_byte = byte;
        self.current_task = CONSOLE_TASK;
        Ok(())
    }

    fn console_reply_and_revoke(&mut self) -> Result<(u8, usize), RuntimeError> {
        if self.current_task != CONSOLE_TASK {
            return Err(RuntimeError::WrongTask);
        }
        authorize_mmio(&self.capabilities, CONSOLE_TASK, UART_SLOT, Rights::WRITE)?;
        let byte = self.pending_byte;
        self.current_task = APP_TASK;
        let revoked = self.capabilities.revoke(ROOT_TASK, ENDPOINT_SLOT)?;
        Ok((byte, revoked))
    }
}

struct RuntimeCell(UnsafeCell<Runtime>);
unsafe impl Sync for RuntimeCell {}
static RUNTIME: RuntimeCell = RuntimeCell(UnsafeCell::new(Runtime::empty()));

fn with_runtime<T>(operation: impl FnOnce(&mut Runtime) -> T) -> T {
    // Aegis is single-core at this milestone and calls this only with IRQs masked
    // or from non-nested synchronous exception handling.
    unsafe { operation(&mut *RUNTIME.0.get()) }
}

pub fn initialize() -> Result<(), RuntimeError> {
    with_runtime(|runtime| {
        *runtime = Runtime::empty();
        runtime.bootstrap()
    })
}

pub fn app_call(byte: u8) -> Result<(), RuntimeError> {
    with_runtime(|runtime| runtime.app_call(byte))
}

pub fn console_reply_and_revoke() -> Result<(u8, usize), RuntimeError> {
    with_runtime(Runtime::console_reply_and_revoke)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_call_succeeds_then_root_revocation_blocks_retry() {
        let mut runtime = Runtime::empty();
        runtime.bootstrap().unwrap();
        runtime.app_call(b'A').unwrap();
        assert_eq!(runtime.console_reply_and_revoke(), Ok((b'A', 2)));
        assert!(matches!(
            runtime.app_call(b'B'),
            Err(RuntimeError::Syscall(SyscallError::Capability(
                CapError::EmptySlot
            )))
        ));
    }
}
