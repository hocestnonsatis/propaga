use priority_queue::PriorityQueue;
use propaga_core::PropagatorId;
use propaga_core::id::PropagatorKey;
use std::cmp::Reverse;

/// Priority queue of propagators scheduled for execution.
pub struct EventQueue {
    queue: PriorityQueue<PropagatorKey, Reverse<(u32, u64)>>,
    counter: u64,
}

impl EventQueue {
    /// Creates an empty event queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: PriorityQueue::new(),
            counter: 0,
        }
    }

    /// Enqueues `propagator` with the given priority.
    pub fn enqueue(&mut self, propagator: PropagatorId, priority: u32) {
        self.counter = self.counter.wrapping_add(1);
        self.queue
            .push(propagator.key(), Reverse((priority, self.counter)));
    }

    /// Removes and returns the next scheduled propagator, if any.
    pub fn pop(&mut self) -> Option<PropagatorId> {
        self.queue.pop().map(|(key, _)| PropagatorId::from_key(key))
    }

    /// Returns `true` when no propagators are scheduled.
    #[must_use]
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Discards all pending events.
    pub fn clear(&mut self) {
        self.queue.clear();
    }
}

impl Default for EventQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use propaga_core::PropagatorId;
    use slotmap::SlotMap;

    #[test]
    fn lower_priority_runs_first() {
        let mut queue = EventQueue::new();
        let mut propagators: SlotMap<PropagatorKey, ()> = SlotMap::with_key();
        let low = PropagatorId::from_key(propagators.insert(()));
        let high = PropagatorId::from_key(propagators.insert(()));

        queue.enqueue(high, 10);
        queue.enqueue(low, 1);

        assert_eq!(queue.pop(), Some(low));
        assert_eq!(queue.pop(), Some(high));
        assert_eq!(queue.pop(), None);
    }
}
