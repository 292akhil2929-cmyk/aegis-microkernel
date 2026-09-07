//! Fixed-priority-free round-robin scheduling policy and saved task context.

pub type TaskId = usize;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UserContext {
    pub x: [u64; 31],
    pub sp_el0: u64,
    pub elr_el1: u64,
    pub spsr_el1: u64,
    pub ttbr0_el1: u64,
}

impl UserContext {
    pub const fn empty() -> Self {
        Self {
            x: [0; 31],
            sp_el0: 0,
            elr_el1: 0,
            spsr_el1: 0,
            ttbr0_el1: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskState {
    Unused,
    Runnable,
    Blocked,
    Faulted,
}

#[derive(Clone, Copy)]
struct Task {
    state: TaskState,
    context: UserContext,
    runtime_ticks: u64,
}

const EMPTY_TASK: Task = Task {
    state: TaskState::Unused,
    context: UserContext::empty(),
    runtime_ticks: 0,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduleError {
    NoTaskSlot,
    BadTask,
    NotRunnable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Switch {
    pub previous: Option<TaskId>,
    pub next: TaskId,
}

pub struct Scheduler<const TASKS: usize> {
    tasks: [Task; TASKS],
    current: Option<TaskId>,
}

impl<const TASKS: usize> Scheduler<TASKS> {
    pub const fn new() -> Self {
        Self {
            tasks: [EMPTY_TASK; TASKS],
            current: None,
        }
    }

    pub fn spawn(&mut self, context: UserContext) -> Result<TaskId, ScheduleError> {
        let id = self
            .tasks
            .iter()
            .position(|task| task.state == TaskState::Unused)
            .ok_or(ScheduleError::NoTaskSlot)?;
        self.tasks[id] = Task {
            state: TaskState::Runnable,
            context,
            runtime_ticks: 0,
        };
        Ok(id)
    }

    pub fn schedule(&mut self) -> Option<Switch> {
        if let Some(current) = self.current {
            if self.tasks[current].state == TaskState::Runnable {
                self.tasks[current].runtime_ticks += 1;
            }
        }
        let start = self.current.map_or(0, |id| (id + 1) % TASKS);
        for offset in 0..TASKS {
            let candidate = (start + offset) % TASKS;
            if self.tasks[candidate].state == TaskState::Runnable {
                let switch = Switch {
                    previous: self.current,
                    next: candidate,
                };
                self.current = Some(candidate);
                return Some(switch);
            }
        }
        self.current = None;
        None
    }

    pub fn block(&mut self, task: TaskId) -> Result<(), ScheduleError> {
        self.set_state(task, TaskState::Blocked)
    }

    pub fn wake(&mut self, task: TaskId) -> Result<(), ScheduleError> {
        let entry = self.tasks.get_mut(task).ok_or(ScheduleError::BadTask)?;
        if entry.state == TaskState::Unused {
            return Err(ScheduleError::BadTask);
        }
        entry.state = TaskState::Runnable;
        Ok(())
    }

    pub fn fault(&mut self, task: TaskId) -> Result<(), ScheduleError> {
        self.set_state(task, TaskState::Faulted)
    }

    pub fn state(&self, task: TaskId) -> Option<TaskState> {
        self.tasks.get(task).map(|entry| entry.state)
    }

    pub fn runtime_ticks(&self, task: TaskId) -> Option<u64> {
        self.tasks.get(task).map(|entry| entry.runtime_ticks)
    }

    pub fn context(&self, task: TaskId) -> Option<&UserContext> {
        self.tasks.get(task).map(|entry| &entry.context)
    }

    fn set_state(&mut self, task: TaskId, state: TaskState) -> Result<(), ScheduleError> {
        let entry = self.tasks.get_mut(task).ok_or(ScheduleError::BadTask)?;
        if entry.state == TaskState::Unused {
            return Err(ScheduleError::BadTask);
        }
        entry.state = state;
        Ok(())
    }
}

impl<const TASKS: usize> Default for Scheduler<TASKS> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_robin_is_fair_and_skips_blocked_tasks() {
        let mut scheduler = Scheduler::<4>::new();
        let a = scheduler.spawn(UserContext::empty()).unwrap();
        let b = scheduler.spawn(UserContext::empty()).unwrap();
        let c = scheduler.spawn(UserContext::empty()).unwrap();
        assert_eq!(scheduler.schedule().unwrap().next, a);
        assert_eq!(scheduler.schedule().unwrap().next, b);
        scheduler.block(c).unwrap();
        assert_eq!(scheduler.schedule().unwrap().next, a);
        assert_eq!(scheduler.schedule().unwrap().next, b);
        scheduler.wake(c).unwrap();
        assert_eq!(scheduler.schedule().unwrap().next, c);
    }

    #[test]
    fn no_runnable_task_returns_idle() {
        let mut scheduler = Scheduler::<1>::new();
        let task = scheduler.spawn(UserContext::empty()).unwrap();
        scheduler.block(task).unwrap();
        assert_eq!(scheduler.schedule(), None);
    }
}
