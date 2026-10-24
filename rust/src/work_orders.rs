//! Durable refinery work orders. Absolute UTC times; no countdown state is persisted.
use crate::{Bookmark, refineries::Load, storage};
use serde::{Deserialize, Serialize};
use std::path::Path;
#[derive(Clone, Serialize, Deserialize)]
pub struct Batch {
    pub input: Load,
    pub output_scu: Option<f64>,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum OutputSource {
    Unknown,
    TerminalQuote,
    ManualEstimate,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: u64,
    pub refinery_id: u64,
    pub refinery: String,
    pub name: String,
    pub approach: Option<Bookmark>,
    pub method: String,
    pub fee: Option<f64>,
    pub batches: Vec<Batch>,
    pub output_source: OutputSource,
    pub recovery_percent: Option<f64>,
    pub created_utc: i64,
    pub completes_utc: i64,
    pub advance_seconds: i64,
    pub sound: bool,
    pub collected_utc: Option<i64>,
    pub advance_notified: bool,
    pub ready_notified: bool,
    pub snooze_until: Option<i64>,
}
impl Job {
    pub fn ready(&self, now: i64) -> bool {
        self.collected_utc.is_none() && now >= self.completes_utc
    }
    pub fn output(&self) -> Option<f64> {
        if self.output_source == OutputSource::Unknown || self.batches.is_empty() {
            return None;
        }
        self.batches
            .iter()
            .try_fold(0., |sum, b| Some(sum + b.output_scu?))
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.id == 0
            || self.name.len() > 200
            || self.refinery.is_empty()
            || self.refinery.len() > 250
            || self.method.len() > 100
            || self.batches.len() > 100
            || !(0..=4102444800).contains(&self.completes_utc)
            || !(0..=604800).contains(&self.advance_seconds)
        {
            return Err("Invalid work order fields".into());
        }
        for b in &self.batches {
            if b.input.ore.len() > 100
                || !b.input.scu.is_finite()
                || !(0.0..=1e6).contains(&b.input.scu)
                || b.input
                    .quality
                    .is_some_and(|q| !q.is_finite() || !(0.0..=1000.).contains(&q))
                || b.output_scu
                    .is_some_and(|q| !q.is_finite() || q < 0. || q > 1e6)
            {
                return Err("Invalid ore quantity or quality".into());
            }
        }
        if self.fee.is_some_and(|x| !x.is_finite() || x < 0.)
            || self
                .recovery_percent
                .is_some_and(|x| !x.is_finite() || !(0.0..=100.).contains(&x))
        {
            return Err("Invalid fee or recovery percentage".into());
        }
        if let Some(b) = &self.approach {
            b.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Queue {
    pub jobs: Vec<Job>,
}
impl Queue {
    pub fn validate(&self) -> Result<(), String> {
        if self.jobs.len() > 5000 {
            return Err("Work order limit reached".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for j in &self.jobs {
            j.validate()?;
            if !ids.insert(j.id) {
                return Err("Duplicate work order ID".into());
            }
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        match std::fs::metadata(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.to_string()),
            Ok(m) if m.len() > 10_000_000 => return Err("Work order file too large".into()),
            _ => (),
        }
        let q: Self = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        q.validate()?;
        Ok(q)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        storage::write_atomic(
            path,
            &serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?,
        )
    }
    pub fn next_id(&self) -> u64 {
        self.jobs.iter().map(|j| j.id).max().unwrap_or(0) + 1
    }
    /// Mutate notification flags in a candidate copy; caller saves before emitting reminders.
    pub fn due(&mut self, now: i64) -> Vec<(u64, bool)> {
        let mut due = Vec::new();
        for j in &mut self.jobs {
            if j.collected_utc.is_some() {
                continue;
            }
            if let Some(snooze) = j.snooze_until {
                if now < snooze {
                    continue;
                }
                j.snooze_until = None;
                j.ready_notified = now >= j.completes_utc;
                j.advance_notified = true;
                due.push((j.id, j.sound));
                continue;
            }
            if now >= j.completes_utc && !j.ready_notified {
                j.ready_notified = true;
                j.advance_notified = true;
                due.push((j.id, j.sound));
            } else if now < j.completes_utc
                && j.advance_seconds > 0
                && now >= j.completes_utc - j.advance_seconds
                && !j.advance_notified
            {
                j.advance_notified = true;
                due.push((j.id, j.sound));
            }
        }
        due
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn job() -> Job {
        Job {
            id: 1,
            refinery_id: 1,
            refinery: "Test".into(),
            name: "Ore".into(),
            approach: None,
            method: String::new(),
            fee: None,
            batches: vec![],
            output_source: OutputSource::Unknown,
            recovery_percent: None,
            created_utc: 0,
            completes_utc: 3600,
            advance_seconds: 600,
            sound: false,
            collected_utc: None,
            advance_notified: false,
            ready_notified: false,
            snooze_until: None,
        }
    }
    #[test]
    fn restart_advance_ready_and_snooze() {
        let mut q = Queue { jobs: vec![job()] };
        assert!(q.due(2999).is_empty());
        assert_eq!(q.due(3000).len(), 1);
        let mut restored: Queue = serde_json::from_slice(&serde_json::to_vec(&q).unwrap()).unwrap();
        assert!(restored.due(3100).is_empty());
        assert_eq!(restored.due(9000).len(), 1);
        assert!(restored.due(9001).is_empty());
        restored.jobs[0].snooze_until = Some(9600);
        assert!(restored.due(9500).is_empty());
        assert_eq!(restored.due(9600).len(), 1);
        restored.jobs[0].collected_utc = Some(9700);
        assert!(restored.due(99999).is_empty());
    }
    #[test]
    fn missed_both_alerts_grouped_once() {
        let mut q = Queue {
            jobs: vec![job(), Job { id: 2, ..job() }],
        };
        assert_eq!(q.due(10000).len(), 2);
        assert!(q.due(10001).is_empty());
    }
    #[test]
    fn unknown_output_never_becomes_zero() {
        assert_eq!(job().output(), None);
    }
}
