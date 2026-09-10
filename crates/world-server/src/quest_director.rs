//! Overlay for generated side-tasks. Vanilla dump data stays canonical.
//!
//! An LLM, if enabled later, may fill [`TaskSpec::flavor`] only. Spawn coordinates,
//! display IDs, loot tables, and combat stay deterministic.

#![allow(dead_code)]

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskObjective {
    Kill { entry: u32, count: u32 },
    Talk { npc_entry: u32 },
    Visit { x: i32, y: i32, z: i32 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskReward {
    pub copper: u32,
    pub item_id: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSpec {
    pub objective: TaskObjective,
    pub reward: TaskReward,
    pub flavor: String,
}

/// Picks a constrained task from nearby creature entries. No LLM in the tick path.
pub struct QuestDirector;

impl QuestDirector {
    pub fn suggest(level: i32, nearby_entries: &[u32]) -> Option<TaskSpec> {
        let entry = *nearby_entries.iter().max()?;
        Some(TaskSpec {
            objective: TaskObjective::Kill {
                entry,
                count: if level < 5 { 4 } else { 8 },
            },
            reward: TaskReward {
                copper: 50,
                item_id: None,
            },
            flavor: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_uses_existing_entries_only() {
        let task = QuestDirector::suggest(1, &[299, 6]).expect("task");
        match task.objective {
            TaskObjective::Kill { entry, count } => {
                assert_eq!(entry, 299);
                assert_eq!(count, 4);
            }
            other => panic!("expected kill, got {other:?}"),
        }
        assert!(task.flavor.is_empty());
    }
}
