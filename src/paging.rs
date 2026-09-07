//! AArch64 stage-1 descriptor construction and fixed mapping records.

use crate::memory::PAGE_SIZE;

pub const ENTRIES_PER_TABLE: usize = 512;
pub const USER_TOP: u64 = 0x0000_0080_0000_0000;

const DESCRIPTOR_VALID: u64 = 1;
const DESCRIPTOR_PAGE: u64 = 1 << 1;
const ACCESS_FLAG: u64 = 1 << 10;
const SH_INNER: u64 = 0b11 << 8;
const AP_READ_ONLY: u64 = 1 << 7;
const AP_USER: u64 = 1 << 6;
const ATTR_NORMAL: u64 = 1 << 2;
const PXN: u64 = 1 << 53;
const UXN: u64 = 1 << 54;
const OUTPUT_ADDRESS_MASK: u64 = 0x0000_ffff_ffff_f000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PagePermissions {
    pub writable: bool,
    pub executable: bool,
    pub user: bool,
    pub device: bool,
}

impl PagePermissions {
    pub const USER_DATA: Self = Self {
        writable: true,
        executable: false,
        user: true,
        device: false,
    };
    pub const USER_CODE: Self = Self {
        writable: false,
        executable: true,
        user: true,
        device: false,
    };
    pub const KERNEL_DATA: Self = Self {
        writable: true,
        executable: false,
        user: false,
        device: false,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MapError {
    Unaligned,
    InvalidAddress,
    WriteExecute,
    AlreadyMapped,
    MapFull,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mapping {
    pub virtual_address: u64,
    pub physical_address: u64,
    pub permissions: PagePermissions,
}

/// Policy-level address-space map used before hardware-table materialization.
pub struct AddressSpace<const MAPPINGS: usize> {
    mappings: [Option<Mapping>; MAPPINGS],
}

impl<const MAPPINGS: usize> AddressSpace<MAPPINGS> {
    pub const fn new() -> Self {
        Self {
            mappings: [None; MAPPINGS],
        }
    }

    pub fn map(&mut self, mapping: Mapping) -> Result<(), MapError> {
        if mapping.virtual_address % PAGE_SIZE != 0 || mapping.physical_address % PAGE_SIZE != 0 {
            return Err(MapError::Unaligned);
        }
        if mapping.virtual_address >= USER_TOP && mapping.permissions.user {
            return Err(MapError::InvalidAddress);
        }
        if mapping.permissions.writable && mapping.permissions.executable {
            return Err(MapError::WriteExecute);
        }
        if self
            .mappings
            .iter()
            .flatten()
            .any(|entry| entry.virtual_address == mapping.virtual_address)
        {
            return Err(MapError::AlreadyMapped);
        }
        let slot = self
            .mappings
            .iter_mut()
            .find(|entry| entry.is_none())
            .ok_or(MapError::MapFull)?;
        *slot = Some(mapping);
        Ok(())
    }

    pub fn mapping_at(&self, virtual_address: u64) -> Option<Mapping> {
        self.mappings
            .iter()
            .flatten()
            .find(|entry| entry.virtual_address == virtual_address)
            .copied()
    }
}

impl<const MAPPINGS: usize> Default for AddressSpace<MAPPINGS> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn level_index(virtual_address: u64, level: u8) -> Result<usize, MapError> {
    let shift = match level {
        0 => 39,
        1 => 30,
        2 => 21,
        3 => 12,
        _ => return Err(MapError::InvalidAddress),
    };
    Ok(((virtual_address >> shift) & 0x1ff) as usize)
}

pub fn page_descriptor(
    physical_address: u64,
    permissions: PagePermissions,
) -> Result<u64, MapError> {
    if physical_address % PAGE_SIZE != 0 {
        return Err(MapError::Unaligned);
    }
    if permissions.writable && permissions.executable {
        return Err(MapError::WriteExecute);
    }
    let mut descriptor =
        (physical_address & OUTPUT_ADDRESS_MASK) | DESCRIPTOR_VALID | DESCRIPTOR_PAGE | ACCESS_FLAG;
    if !permissions.device {
        descriptor |= ATTR_NORMAL | SH_INNER;
    }
    if !permissions.writable {
        descriptor |= AP_READ_ONLY;
    }
    if permissions.user {
        descriptor |= AP_USER;
    }
    if !permissions.executable {
        descriptor |= PXN | UXN;
    } else if permissions.user {
        descriptor |= PXN;
    } else {
        descriptor |= UXN;
    }
    Ok(descriptor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_kibibyte_indices_match_arm_bit_slices() {
        let address = 0x0000_1234_5678_9000;
        assert_eq!(
            level_index(address, 0),
            Ok(((address >> 39) & 0x1ff) as usize)
        );
        assert_eq!(
            level_index(address, 3),
            Ok(((address >> 12) & 0x1ff) as usize)
        );
    }

    #[test]
    fn user_code_is_read_only_and_unprivileged_execute_only() {
        let descriptor = page_descriptor(0x4000_0000, PagePermissions::USER_CODE).unwrap();
        assert_ne!(descriptor & AP_READ_ONLY, 0);
        assert_ne!(descriptor & AP_USER, 0);
        assert_ne!(descriptor & PXN, 0);
        assert_eq!(descriptor & UXN, 0);
    }

    #[test]
    fn writable_executable_and_duplicate_maps_are_rejected() {
        let mut space = AddressSpace::<2>::new();
        let data = Mapping {
            virtual_address: 0x1000,
            physical_address: 0x4000_0000,
            permissions: PagePermissions::USER_DATA,
        };
        space.map(data).unwrap();
        assert_eq!(space.map(data), Err(MapError::AlreadyMapped));
        assert_eq!(
            page_descriptor(
                0x4000_1000,
                PagePermissions {
                    writable: true,
                    executable: true,
                    user: true,
                    device: false,
                }
            ),
            Err(MapError::WriteExecute)
        );
    }
}
