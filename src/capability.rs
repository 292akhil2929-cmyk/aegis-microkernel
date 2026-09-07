//! Aegis' small, auditable capability model.
//!
//! User space names a slot, never a kernel object address. The kernel resolves
//! that slot through the caller's CSpace. Derived nodes form a revocation tree.

pub type TaskId = usize;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Object {
    Endpoint(u16),
    Frame(u32),
    Thread(TaskId),
    Irq(u16),
    Mmio { base: u64, pages: u16 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rights(u8);

impl Rights {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const GRANT: Self = Self(1 << 2);
    pub const CONTROL: Self = Self(1 << 3);
    pub const ALL: Self = Self(0x0f);

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains(self, requested: Self) -> bool {
        self.0 & requested.0 == requested.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapRef {
    node: u16,
    generation: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResolvedCapability {
    pub object: Object,
    pub rights: Rights,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapError {
    BadTask,
    BadSlot,
    EmptySlot,
    DestinationOccupied,
    OutOfNodes,
    Stale,
    PermissionDenied,
}

#[derive(Clone, Copy)]
struct Node {
    object: Object,
    rights: Rights,
    parent: Option<CapRef>,
    generation: u16,
    live: bool,
}

const EMPTY_NODE: Node = Node {
    object: Object::Endpoint(0),
    rights: Rights::NONE,
    parent: None,
    generation: 0,
    live: false,
};

/// Fixed-size capability state: predictable memory use and no kernel allocator.
pub struct CapabilitySystem<const TASKS: usize, const SLOTS: usize, const NODES: usize> {
    spaces: [[Option<CapRef>; SLOTS]; TASKS],
    nodes: [Node; NODES],
}

impl<const TASKS: usize, const SLOTS: usize, const NODES: usize>
    CapabilitySystem<TASKS, SLOTS, NODES>
{
    pub const fn new() -> Self {
        Self {
            spaces: [[None; SLOTS]; TASKS],
            nodes: [EMPTY_NODE; NODES],
        }
    }

    pub fn create_root(
        &mut self,
        owner: TaskId,
        slot: usize,
        object: Object,
        rights: Rights,
    ) -> Result<(), CapError> {
        self.check_destination(owner, slot)?;
        let cap = self.allocate(object, rights, None)?;
        self.spaces[owner][slot] = Some(cap);
        Ok(())
    }

    /// Mint an attenuated child into another task. Rights may only decrease.
    pub fn mint(
        &mut self,
        source_task: TaskId,
        source_slot: usize,
        destination_task: TaskId,
        destination_slot: usize,
        rights: Rights,
    ) -> Result<(), CapError> {
        self.check_destination(destination_task, destination_slot)?;
        let parent = self.cap_at(source_task, source_slot)?;
        let resolved = self.resolve_ref(parent)?;
        if !resolved.rights.contains(Rights::GRANT) || !resolved.rights.contains(rights) {
            return Err(CapError::PermissionDenied);
        }
        let child = self.allocate(resolved.object, rights, Some(parent))?;
        self.spaces[destination_task][destination_slot] = Some(child);
        Ok(())
    }

    pub fn resolve(
        &self,
        task: TaskId,
        slot: usize,
        required: Rights,
    ) -> Result<ResolvedCapability, CapError> {
        let cap = self.cap_at(task, slot)?;
        let resolved = self.resolve_ref(cap)?;
        if !resolved.rights.contains(required) {
            return Err(CapError::PermissionDenied);
        }
        Ok(resolved)
    }

    /// Revoke a capability and every descendant minted from it.
    pub fn revoke(&mut self, task: TaskId, slot: usize) -> Result<usize, CapError> {
        let root = self.cap_at(task, slot)?;
        self.resolve_ref(root)?;

        let mut revoked = 0;
        for index in 0..NODES {
            if self.nodes[index].live && self.is_descendant_or_self(index, root) {
                self.nodes[index].live = false;
                self.nodes[index].generation = self.nodes[index].generation.wrapping_add(1);
                revoked += 1;
            }
        }

        for space in &mut self.spaces {
            for cap in space {
                if let Some(reference) = *cap {
                    let node = &self.nodes[reference.node as usize];
                    if !node.live || node.generation != reference.generation {
                        *cap = None;
                    }
                }
            }
        }
        Ok(revoked)
    }

    fn check_destination(&self, task: TaskId, slot: usize) -> Result<(), CapError> {
        let space = self.spaces.get(task).ok_or(CapError::BadTask)?;
        let entry = space.get(slot).ok_or(CapError::BadSlot)?;
        if entry.is_some() {
            Err(CapError::DestinationOccupied)
        } else {
            Ok(())
        }
    }

    fn cap_at(&self, task: TaskId, slot: usize) -> Result<CapRef, CapError> {
        self.spaces
            .get(task)
            .ok_or(CapError::BadTask)?
            .get(slot)
            .ok_or(CapError::BadSlot)?
            .as_ref()
            .copied()
            .ok_or(CapError::EmptySlot)
    }

    fn resolve_ref(&self, cap: CapRef) -> Result<ResolvedCapability, CapError> {
        let node = self.nodes.get(cap.node as usize).ok_or(CapError::Stale)?;
        if !node.live || node.generation != cap.generation {
            return Err(CapError::Stale);
        }
        Ok(ResolvedCapability {
            object: node.object,
            rights: node.rights,
        })
    }

    fn allocate(
        &mut self,
        object: Object,
        rights: Rights,
        parent: Option<CapRef>,
    ) -> Result<CapRef, CapError> {
        let (index, node) = self
            .nodes
            .iter_mut()
            .enumerate()
            .find(|(_, node)| !node.live)
            .ok_or(CapError::OutOfNodes)?;
        node.object = object;
        node.rights = rights;
        node.parent = parent;
        node.live = true;
        Ok(CapRef {
            node: index as u16,
            generation: node.generation,
        })
    }

    fn is_descendant_or_self(&self, index: usize, root: CapRef) -> bool {
        let mut current = Some(CapRef {
            node: index as u16,
            generation: self.nodes[index].generation,
        });
        let mut depth = 0;
        while let Some(cap) = current {
            if cap == root {
                return true;
            }
            let Some(node) = self.nodes.get(cap.node as usize) else {
                return false;
            };
            current = node.parent;
            depth += 1;
            if depth > NODES {
                return false;
            }
        }
        false
    }
}

impl<const TASKS: usize, const SLOTS: usize, const NODES: usize> Default
    for CapabilitySystem<TASKS, SLOTS, NODES>
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type Caps = CapabilitySystem<4, 8, 16>;

    #[test]
    fn task_cannot_resolve_a_capability_it_was_not_granted() {
        let mut caps = Caps::new();
        caps.create_root(
            0,
            0,
            Object::Mmio {
                base: 0x0900_0000,
                pages: 1,
            },
            Rights::ALL,
        )
        .unwrap();

        assert_eq!(caps.resolve(1, 0, Rights::WRITE), Err(CapError::EmptySlot));
    }

    #[test]
    fn mint_attenuates_rights() {
        let mut caps = Caps::new();
        caps.create_root(0, 0, Object::Endpoint(7), Rights::ALL)
            .unwrap();
        caps.mint(0, 0, 1, 2, Rights::READ).unwrap();

        assert!(caps.resolve(1, 2, Rights::READ).is_ok());
        assert_eq!(
            caps.resolve(1, 2, Rights::WRITE),
            Err(CapError::PermissionDenied)
        );
        assert_eq!(
            caps.mint(1, 2, 2, 0, Rights::READ),
            Err(CapError::PermissionDenied)
        );
    }

    #[test]
    fn revocation_invalidates_the_whole_derivation_subtree() {
        let mut caps = Caps::new();
        caps.create_root(0, 0, Object::Frame(42), Rights::ALL)
            .unwrap();
        caps.mint(0, 0, 1, 0, Rights::READ.union(Rights::GRANT))
            .unwrap();
        caps.mint(1, 0, 2, 0, Rights::READ).unwrap();

        assert_eq!(caps.revoke(0, 0), Ok(3));
        assert_eq!(caps.resolve(1, 0, Rights::READ), Err(CapError::EmptySlot));
        assert_eq!(caps.resolve(2, 0, Rights::READ), Err(CapError::EmptySlot));
    }
}
