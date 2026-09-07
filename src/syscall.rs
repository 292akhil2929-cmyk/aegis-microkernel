//! Capability-authorized syscall decoding.

use crate::capability::{CapError, CapabilitySystem, Object, Rights, TaskId};

#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Syscall {
    Yield = 0,
    Send = 1,
    Receive = 2,
    Call = 3,
    Reply = 4,
    MapFrame = 5,
    Mint = 6,
    Revoke = 7,
}

impl TryFrom<u64> for Syscall {
    type Error = SyscallError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Yield),
            1 => Ok(Self::Send),
            2 => Ok(Self::Receive),
            3 => Ok(Self::Call),
            4 => Ok(Self::Reply),
            5 => Ok(Self::MapFrame),
            6 => Ok(Self::Mint),
            7 => Ok(Self::Revoke),
            _ => Err(SyscallError::Unknown),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyscallError {
    Unknown,
    Capability(CapError),
    WrongObjectType,
}

impl From<CapError> for SyscallError {
    fn from(value: CapError) -> Self {
        Self::Capability(value)
    }
}

pub fn authorize_endpoint<const TASKS: usize, const SLOTS: usize, const NODES: usize>(
    capabilities: &CapabilitySystem<TASKS, SLOTS, NODES>,
    caller: TaskId,
    slot: usize,
    required: Rights,
) -> Result<u16, SyscallError> {
    match capabilities.resolve(caller, slot, required)?.object {
        Object::Endpoint(endpoint) => Ok(endpoint),
        _ => Err(SyscallError::WrongObjectType),
    }
}

pub fn authorize_frame<const TASKS: usize, const SLOTS: usize, const NODES: usize>(
    capabilities: &CapabilitySystem<TASKS, SLOTS, NODES>,
    caller: TaskId,
    slot: usize,
    required: Rights,
) -> Result<u32, SyscallError> {
    match capabilities.resolve(caller, slot, required)?.object {
        Object::Frame(frame) => Ok(frame),
        _ => Err(SyscallError::WrongObjectType),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn syscall_numbers_are_strictly_decoded() {
        assert_eq!(Syscall::try_from(3), Ok(Syscall::Call));
        assert_eq!(Syscall::try_from(99), Err(SyscallError::Unknown));
    }

    #[test]
    fn endpoint_and_frame_authority_are_typed() {
        let mut caps = CapabilitySystem::<2, 4, 8>::new();
        caps.create_root(0, 0, Object::Endpoint(4), Rights::ALL)
            .unwrap();
        caps.create_root(0, 1, Object::Frame(9), Rights::READ)
            .unwrap();
        assert_eq!(authorize_endpoint(&caps, 0, 0, Rights::WRITE), Ok(4));
        assert_eq!(
            authorize_frame(&caps, 0, 0, Rights::READ),
            Err(SyscallError::WrongObjectType)
        );
        assert_eq!(
            authorize_frame(&caps, 0, 1, Rights::WRITE),
            Err(SyscallError::Capability(CapError::PermissionDenied))
        );
        assert_eq!(
            authorize_endpoint(&caps, 1, 0, Rights::READ),
            Err(SyscallError::Capability(CapError::EmptySlot))
        );
    }
}
