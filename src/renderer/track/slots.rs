use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Condvar, Mutex};

/// Limits actual synthesis work independently from the Rayon pool. This keeps
/// external UTAU engines from spawning more concurrent processes than their
/// voicebank caches and the host machine can sustain.
pub(super) struct ResamplerSlots {
    available: Mutex<usize>,
    ready: Condvar,
}

pub(super) struct ResamplerSlot<'a> {
    slots: &'a ResamplerSlots,
}

impl ResamplerSlots {
    pub(super) fn new(instances: u32) -> Self {
        Self {
            available: Mutex::new(instances.max(1) as usize),
            ready: Condvar::new(),
        }
    }

    pub(super) fn acquire(&self, cancel: Option<&AtomicBool>) -> Result<ResamplerSlot<'_>, String> {
        loop {
            if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                return Err("renderização cancelada".to_string());
            }
            let mut available = self
                .available
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if *available > 0 {
                *available -= 1;
                return Ok(ResamplerSlot { slots: self });
            }
            let (next, _) = self
                .ready
                .wait_timeout(available, std::time::Duration::from_millis(20))
                .unwrap_or_else(|error| error.into_inner());
            drop(next);
        }
    }
}

impl Drop for ResamplerSlot<'_> {
    fn drop(&mut self) {
        let mut available = self
            .slots
            .available
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *available += 1;
        self.slots.ready.notify_one();
    }
}
