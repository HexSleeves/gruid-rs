//! Priority event queue for scheduling game events.
//!
//! Events are stored in a min-heap keyed by `(rank, insertion_order)`.
//! Lower ranks are popped first; ties are broken by insertion order
//! (FIFO).

use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// An entry in the event queue.
#[derive(Debug)]
struct Entry<E> {
    event: E,
    rank: i32,
    /// Monotonically increasing counter used to break ties.
    /// Lower = inserted earlier = higher priority for FIFO.
    seq: u64,
    /// If true, this entry should sort before equal-rank entries
    /// with `first == false` (by having a lower effective seq).
    first: bool,
}

impl<E> PartialEq for Entry<E> {
    fn eq(&self, other: &Self) -> bool {
        self.rank == other.rank && self.seq == other.seq
    }
}

impl<E> Eq for Entry<E> {}

impl<E> PartialOrd for Entry<E> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<E> Ord for Entry<E> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // We wrap in Reverse for the BinaryHeap, so this ordering
        // is for "natural" comparison: smaller rank first, then smaller seq.
        self.rank.cmp(&other.rank).then_with(|| {
            // Entries with `first == true` should come before others
            // at the same rank. Among `first` entries, use seq.
            // Among non-first entries, use seq.
            match (self.first, other.first) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => self.seq.cmp(&other.seq),
            }
        })
    }
}

/// A priority event queue.
///
/// Events with lower rank are dequeued first. Among events with the same
/// rank, those pushed earlier are dequeued first (FIFO).
/// [`push_first`](Self::push_first) entries at a given rank come before
/// [`push`](Self::push) entries at the same rank.
pub struct EventQueue<E> {
    heap: BinaryHeap<Reverse<Entry<E>>>,
    seq: u64,
}

impl<E> EventQueue<E> {
    /// Create an empty event queue.
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            seq: 0,
        }
    }

    /// Push an event at the given rank.
    pub fn push(&mut self, event: E, rank: i32) {
        let seq = self.seq;
        self.seq += 1;
        self.heap.push(Reverse(Entry {
            event,
            rank,
            seq,
            first: false,
        }));
    }

    /// Push an event at the given rank with FIFO-first semantics.
    ///
    /// At equal rank, events pushed with `push_first` are dequeued
    /// before events pushed with `push`.
    pub fn push_first(&mut self, event: E, rank: i32) {
        let seq = self.seq;
        self.seq += 1;
        self.heap.push(Reverse(Entry {
            event,
            rank,
            seq,
            first: true,
        }));
    }

    /// Pop the event with the lowest rank (ties broken FIFO).
    pub fn pop(&mut self) -> Option<E> {
        self.heap.pop().map(|Reverse(entry)| entry.event)
    }

    /// Pop the event with the lowest rank, also returning the rank.
    pub fn pop_with_rank(&mut self) -> Option<(E, i32)> {
        self.heap
            .pop()
            .map(|Reverse(entry)| (entry.event, entry.rank))
    }

    /// Whether the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Number of events in the queue.
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Remove all events that do **not** satisfy the predicate.
    ///
    /// Events for which `predicate` returns `false` are removed.
    pub fn filter(&mut self, predicate: impl Fn(&E) -> bool) {
        let old_heap = std::mem::take(&mut self.heap);
        self.heap = old_heap
            .into_iter()
            .filter(|Reverse(entry)| predicate(&entry.event))
            .collect();
    }
}

impl<E> Default for EventQueue<E> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_push_pop() {
        let mut q = EventQueue::new();
        q.push("a", 3);
        q.push("b", 1);
        q.push("c", 2);

        assert_eq!(q.pop(), Some("b"));
        assert_eq!(q.pop(), Some("c"));
        assert_eq!(q.pop(), Some("a"));
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn test_fifo_same_rank() {
        let mut q = EventQueue::new();
        q.push("first", 1);
        q.push("second", 1);
        q.push("third", 1);

        assert_eq!(q.pop(), Some("first"));
        assert_eq!(q.pop(), Some("second"));
        assert_eq!(q.pop(), Some("third"));
    }

    #[test]
    fn test_push_first_priority() {
        let mut q = EventQueue::new();
        q.push("normal", 1);
        q.push_first("first", 1);

        // push_first event should come out before push event at same rank.
        assert_eq!(q.pop(), Some("first"));
        assert_eq!(q.pop(), Some("normal"));
    }

    #[test]
    fn test_pop_with_rank() {
        let mut q = EventQueue::new();
        q.push(42, 5);
        assert_eq!(q.pop_with_rank(), Some((42, 5)));
    }

    #[test]
    fn test_filter() {
        let mut q = EventQueue::new();
        q.push(1, 1);
        q.push(2, 2);
        q.push(3, 3);
        q.push(4, 4);

        q.filter(|e| *e % 2 == 0);
        assert_eq!(q.len(), 2);
        assert_eq!(q.pop(), Some(2));
        assert_eq!(q.pop(), Some(4));
    }

    #[test]
    fn test_is_empty_and_len() {
        let mut q = EventQueue::<i32>::new();
        assert!(q.is_empty());
        assert_eq!(q.len(), 0);

        q.push(1, 1);
        assert!(!q.is_empty());
        assert_eq!(q.len(), 1);

        q.pop();
        assert!(q.is_empty());
    }
}
