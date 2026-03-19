//! Job queue — durable task execution with priority and deduplication.
//!
//! In production, backed by Redis (Upstash). For local development,
//! uses an in-memory priority queue.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A job in the queue.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct Job {
    /// Unique job ID
    pub id: String,
    /// Associated task ID
    pub task_id: String,
    /// Agent role to execute this job
    pub agent_role: String,
    /// Priority (higher = more important)
    pub priority: u32,
    /// Job payload
    pub payload: serde_json::Value,
    /// When the job was enqueued
    pub enqueued_at: DateTime<Utc>,
    /// Current status
    pub status: JobStatus,
    /// Number of processing attempts
    pub attempts: u32,
    /// Maximum processing attempts
    pub max_attempts: u32,
}

impl Job {
    pub fn new(
        task_id: impl Into<String>,
        agent_role: impl Into<String>,
        priority: u32,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_id: task_id.into(),
            agent_role: agent_role.into(),
            priority,
            payload,
            enqueued_at: Utc::now(),
            status: JobStatus::Queued,
            attempts: 0,
            max_attempts: 3,
        }
    }
}

/// Priority ordering for the heap (higher priority first, then FIFO by time).
impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.enqueued_at.cmp(&self.enqueued_at)) // Earlier first
    }
}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    DeadLetter,
}

/// In-memory priority job queue.
pub struct JobQueue {
    heap: BinaryHeap<Job>,
    /// Track jobs by ID for status lookups
    jobs: HashMap<String, Job>,
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            jobs: HashMap::new(),
        }
    }

    /// Enqueue a job.
    pub fn enqueue(&mut self, job: Job) -> String {
        let id = job.id.clone();
        self.jobs.insert(id.clone(), job.clone());
        self.heap.push(job);
        id
    }

    /// Dequeue the highest-priority job.
    pub fn dequeue(&mut self) -> Option<Job> {
        while let Some(mut job) = self.heap.pop() {
            // Skip if already processed (dedup)
            if let Some(stored) = self.jobs.get(&job.id) {
                if stored.status != JobStatus::Queued {
                    continue;
                }
            }

            job.status = JobStatus::Processing;
            job.attempts += 1;
            self.jobs.insert(job.id.clone(), job.clone());
            return Some(job);
        }
        None
    }

    /// Mark a job as completed.
    pub fn complete(&mut self, job_id: &str) {
        if let Some(job) = self.jobs.get_mut(job_id) {
            job.status = JobStatus::Completed;
        }
    }

    /// Mark a job as failed. Re-queues if under max_attempts, otherwise dead-letters.
    pub fn fail(&mut self, job_id: &str) {
        if let Some(job) = self.jobs.get_mut(job_id) {
            if job.attempts < job.max_attempts {
                job.status = JobStatus::Queued;
                self.heap.push(job.clone());
            } else {
                job.status = JobStatus::DeadLetter;
            }
        }
    }

    /// Get a job by ID.
    pub fn get(&self, job_id: &str) -> Option<&Job> {
        self.jobs.get(job_id)
    }

    /// Number of queued jobs.
    pub fn queued_count(&self) -> usize {
        self.jobs
            .values()
            .filter(|j| j.status == JobStatus::Queued)
            .count()
    }

    /// Number of dead-lettered jobs.
    pub fn dead_letter_count(&self) -> usize {
        self.jobs
            .values()
            .filter(|j| j.status == JobStatus::DeadLetter)
            .count()
    }

    /// Get all dead-lettered jobs.
    pub fn dead_letters(&self) -> Vec<&Job> {
        self.jobs
            .values()
            .filter(|j| j.status == JobStatus::DeadLetter)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enqueue_dequeue() {
        let mut q = JobQueue::new();
        let job = Job::new("t1", "backend", 1, serde_json::Value::Null);
        q.enqueue(job);

        let dequeued = q.dequeue().unwrap();
        assert_eq!(dequeued.task_id, "t1");
        assert_eq!(dequeued.status, JobStatus::Processing);
    }

    #[test]
    fn test_priority_ordering() {
        let mut q = JobQueue::new();
        q.enqueue(Job::new("low", "backend", 1, serde_json::Value::Null));
        q.enqueue(Job::new("high", "cto", 10, serde_json::Value::Null));
        q.enqueue(Job::new("mid", "frontend", 5, serde_json::Value::Null));

        assert_eq!(q.dequeue().unwrap().task_id, "high");
        assert_eq!(q.dequeue().unwrap().task_id, "mid");
        assert_eq!(q.dequeue().unwrap().task_id, "low");
    }

    #[test]
    fn test_fail_and_retry() {
        let mut q = JobQueue::new();
        let job = Job::new("t1", "backend", 1, serde_json::Value::Null);
        let id = q.enqueue(job);

        let dequeued = q.dequeue().unwrap();
        assert_eq!(dequeued.attempts, 1);

        q.fail(&id);

        // Should be re-queued
        let retry = q.dequeue().unwrap();
        assert_eq!(retry.attempts, 2);
    }

    #[test]
    fn test_dead_letter_after_max_attempts() {
        let mut q = JobQueue::new();
        let mut job = Job::new("t1", "backend", 1, serde_json::Value::Null);
        job.max_attempts = 2;
        let id = q.enqueue(job);

        q.dequeue(); // attempt 1
        q.fail(&id);
        q.dequeue(); // attempt 2
        q.fail(&id);

        assert_eq!(q.dead_letter_count(), 1);
        assert!(q.dequeue().is_none());
    }

    #[test]
    fn test_complete() {
        let mut q = JobQueue::new();
        let job = Job::new("t1", "backend", 1, serde_json::Value::Null);
        let id = q.enqueue(job);

        q.dequeue();
        q.complete(&id);

        assert_eq!(q.get(&id).unwrap().status, JobStatus::Completed);
    }
}
