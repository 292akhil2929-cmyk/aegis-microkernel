//! Synchronous, zero-kernel-buffer rendezvous IPC model.

pub type ThreadId = usize;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Message {
    pub label: u16,
    pub length: u8,
    pub registers: [u64; 4],
    /// Optional sender CSpace slot to mint into a receiver-selected slot.
    pub capability_slot: Option<u16>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ThreadState {
    Runnable,
    Sending { endpoint: usize },
    Receiving { endpoint: usize },
}

#[derive(Clone, Copy)]
struct Thread {
    state: ThreadState,
    outgoing: Message,
    inbox: Option<Message>,
}

const EMPTY_THREAD: Thread = Thread {
    state: ThreadState::Runnable,
    outgoing: Message {
        label: 0,
        length: 0,
        registers: [0; 4],
        capability_slot: None,
    },
    inbox: None,
};

#[derive(Clone, Copy)]
struct Queue<const THREADS: usize> {
    entries: [Option<ThreadId>; THREADS],
    head: usize,
    len: usize,
}

impl<const THREADS: usize> Queue<THREADS> {
    const fn new() -> Self {
        Self {
            entries: [None; THREADS],
            head: 0,
            len: 0,
        }
    }

    fn push(&mut self, thread: ThreadId) -> Result<(), IpcError> {
        if self.len == THREADS {
            return Err(IpcError::QueueFull);
        }
        let tail = (self.head + self.len) % THREADS;
        self.entries[tail] = Some(thread);
        self.len += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<ThreadId> {
        if self.len == 0 {
            return None;
        }
        let value = self.entries[self.head].take();
        self.head = (self.head + 1) % THREADS;
        self.len -= 1;
        value
    }
}

#[derive(Clone, Copy)]
struct Endpoint<const THREADS: usize> {
    senders: Queue<THREADS>,
    receivers: Queue<THREADS>,
}

impl<const THREADS: usize> Endpoint<THREADS> {
    const fn new() -> Self {
        Self {
            senders: Queue::new(),
            receivers: Queue::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcOutcome {
    Blocked,
    Delivered { peer: ThreadId },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpcError {
    BadThread,
    BadEndpoint,
    ThreadNotRunnable,
    QueueFull,
}

pub struct RendezvousIpc<const THREADS: usize, const ENDPOINTS: usize> {
    threads: [Thread; THREADS],
    endpoints: [Endpoint<THREADS>; ENDPOINTS],
}

impl<const THREADS: usize, const ENDPOINTS: usize> RendezvousIpc<THREADS, ENDPOINTS> {
    pub const fn new() -> Self {
        Self {
            threads: [EMPTY_THREAD; THREADS],
            endpoints: [Endpoint::new(); ENDPOINTS],
        }
    }

    pub fn send(
        &mut self,
        sender: ThreadId,
        endpoint: usize,
        message: Message,
    ) -> Result<IpcOutcome, IpcError> {
        self.ensure_runnable(sender)?;
        let ep = self
            .endpoints
            .get_mut(endpoint)
            .ok_or(IpcError::BadEndpoint)?;
        if let Some(receiver) = ep.receivers.pop() {
            self.threads[receiver].inbox = Some(message);
            self.threads[receiver].state = ThreadState::Runnable;
            Ok(IpcOutcome::Delivered { peer: receiver })
        } else {
            self.threads[sender].outgoing = message;
            self.threads[sender].state = ThreadState::Sending { endpoint };
            ep.senders.push(sender)?;
            Ok(IpcOutcome::Blocked)
        }
    }

    pub fn receive(&mut self, receiver: ThreadId, endpoint: usize) -> Result<IpcOutcome, IpcError> {
        self.ensure_runnable(receiver)?;
        let ep = self
            .endpoints
            .get_mut(endpoint)
            .ok_or(IpcError::BadEndpoint)?;
        if let Some(sender) = ep.senders.pop() {
            let message = self.threads[sender].outgoing;
            self.threads[sender].state = ThreadState::Runnable;
            self.threads[receiver].inbox = Some(message);
            Ok(IpcOutcome::Delivered { peer: sender })
        } else {
            self.threads[receiver].state = ThreadState::Receiving { endpoint };
            ep.receivers.push(receiver)?;
            Ok(IpcOutcome::Blocked)
        }
    }

    pub fn take_message(&mut self, thread: ThreadId) -> Option<Message> {
        self.threads.get_mut(thread)?.inbox.take()
    }

    pub fn state(&self, thread: ThreadId) -> Option<ThreadState> {
        self.threads.get(thread).map(|thread| thread.state)
    }

    fn ensure_runnable(&self, thread: ThreadId) -> Result<(), IpcError> {
        let state = self.threads.get(thread).ok_or(IpcError::BadThread)?.state;
        if state == ThreadState::Runnable {
            Ok(())
        } else {
            Err(IpcError::ThreadNotRunnable)
        }
    }
}

impl<const THREADS: usize, const ENDPOINTS: usize> Default for RendezvousIpc<THREADS, ENDPOINTS> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sender_blocks_until_receiver_rendezvous() {
        let mut ipc = RendezvousIpc::<4, 2>::new();
        let message = Message {
            label: 9,
            length: 1,
            registers: [0xcafe, 0, 0, 0],
            capability_slot: None,
        };
        assert_eq!(ipc.send(0, 1, message), Ok(IpcOutcome::Blocked));
        assert_eq!(ipc.state(0), Some(ThreadState::Sending { endpoint: 1 }));
        assert_eq!(ipc.receive(1, 1), Ok(IpcOutcome::Delivered { peer: 0 }));
        assert_eq!(ipc.take_message(1), Some(message));
        assert_eq!(ipc.state(0), Some(ThreadState::Runnable));
    }

    #[test]
    fn receiver_can_wait_first() {
        let mut ipc = RendezvousIpc::<4, 1>::new();
        let message = Message {
            label: 1,
            length: 0,
            registers: [0; 4],
            capability_slot: Some(3),
        };
        assert_eq!(ipc.receive(2, 0), Ok(IpcOutcome::Blocked));
        assert_eq!(
            ipc.send(3, 0, message),
            Ok(IpcOutcome::Delivered { peer: 2 })
        );
        assert_eq!(ipc.take_message(2), Some(message));
    }
}
