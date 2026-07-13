//! 密钥池 POC — 加权轮询 + 阶梯冷却（beeapi-switch 同构，Spike S4）

use crate::error::AppError;
use crate::services::simple_connect::key_store::{get_api_key, get_primary_key};
use crate::services::simple_connect::state::load_state;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct PoolKey {
    pub id: String,
    pub label: String,
    pub weight: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
struct KeyRuntime {
    key: PoolKey,
    success: u64,
    failure: u64,
    consecutive_failures: u32,
    cooldown_stage: u32,
    cooling_until: Option<Instant>,
    current_weight: i64,
    last_status: Option<u16>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct KeyPickResult {
    pub key_id: String,
    pub label: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PoolKeyStat {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub weight: u32,
    pub success: u64,
    pub failure: u64,
    pub cooldown_stage: u32,
    pub cooling_remaining_secs: Option<u64>,
    pub last_status: Option<u16>,
    pub available: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolSimulationStep {
    pub action: String,
    pub key_id: String,
    pub detail: String,
}

pub struct KeyPool {
    keys: Vec<KeyRuntime>,
    cursor: usize,
    fail_threshold: u32,
}

fn cooldown_seconds(stage: u32) -> u64 {
    match stage {
        1 => 3,
        2 => 10,
        3 => 30,
        4 => 90,
        _ => 300,
    }
}

impl KeyPool {
    pub fn new(keys: Vec<PoolKey>, fail_threshold: u32) -> Self {
        Self {
            keys: keys
                .into_iter()
                .map(|k| KeyRuntime {
                    key: k,
                    success: 0,
                    failure: 0,
                    consecutive_failures: 0,
                    cooldown_stage: 0,
                    cooling_until: None,
                    current_weight: 0,
                    last_status: None,
                })
                .collect(),
            cursor: 0,
            fail_threshold: fail_threshold.max(1),
        }
    }

    fn eligible_indices(&self, now: Instant) -> Vec<usize> {
        self.keys
            .iter()
            .enumerate()
            .filter(|(_, rt)| rt.available_at(now))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn pick_next(&mut self, now: Instant) -> Option<KeyPickResult> {
        let eligible = self.eligible_indices(now);
        if eligible.is_empty() {
            return None;
        }

        let total_weight: i64 = eligible.iter().map(|&i| self.keys[i].weight_score()).sum();

        for &idx in &eligible {
            self.keys[idx].current_weight += self.keys[idx].weight_score();
        }

        let best = eligible.iter().copied().max_by(|&a, &b| {
            self.keys[a]
                .current_weight
                .cmp(&self.keys[b].current_weight)
                .then_with(|| b.cmp(&a))
        })?;

        self.keys[best].current_weight -= total_weight;
        self.cursor = best;

        Some(KeyPickResult {
            key_id: self.keys[best].key.id.clone(),
            label: self.keys[best].key.label.clone(),
        })
    }

    pub fn record(&mut self, ok: bool, status: Option<u16>, now: Instant) {
        let idx = self.cursor;
        if idx >= self.keys.len() {
            return;
        }
        let rt = &mut self.keys[idx];
        rt.last_status = status;
        if ok {
            rt.success += 1;
            rt.consecutive_failures = 0;
            rt.cooling_until = None;
            rt.cooldown_stage = 0;
        } else {
            rt.failure += 1;
            rt.consecutive_failures += 1;
            if rt.consecutive_failures >= self.fail_threshold {
                let stage = rt.consecutive_failures - self.fail_threshold + 1;
                rt.cooldown_stage = stage;
                rt.cooling_until = Some(now + Duration::from_secs(cooldown_seconds(stage)));
                rt.current_weight = 0;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn cooling_remaining_secs(&self, key_id: &str, now: Instant) -> Option<u64> {
        self.keys.iter().find_map(|rt| {
            if rt.key.id != key_id {
                return None;
            }
            rt.cooling_until.map(|until| {
                if until <= now {
                    0
                } else {
                    (until - now).as_secs()
                }
            })
        })
    }

    pub fn snapshot_stats(&self, now: Instant) -> Vec<PoolKeyStat> {
        self.keys
            .iter()
            .map(|rt| PoolKeyStat {
                id: rt.key.id.clone(),
                label: rt.key.label.clone(),
                enabled: rt.key.enabled,
                weight: rt.key.weight,
                success: rt.success,
                failure: rt.failure,
                cooldown_stage: rt.cooldown_stage,
                cooling_remaining_secs: rt.cooling_until.map(|until| {
                    if until <= now {
                        0
                    } else {
                        (until - now).as_secs()
                    }
                }),
                last_status: rt.last_status,
                available: rt.available_at(now),
            })
            .collect()
    }

    /// Spike S4：模拟 429 触发冷却并切换到另一 Key
    pub fn simulate_failover_demo() -> Vec<PoolSimulationStep> {
        let mut steps = Vec::new();
        let mut pool = KeyPool::new(
            vec![
                PoolKey {
                    id: "k1".into(),
                    label: "key-1".into(),
                    weight: 1,
                    enabled: true,
                },
                PoolKey {
                    id: "k2".into(),
                    label: "key-2".into(),
                    weight: 1,
                    enabled: true,
                },
            ],
            1,
        );
        let t0 = Instant::now();

        if let Some(pick) = pool.pick_next(t0) {
            steps.push(PoolSimulationStep {
                action: "pick".into(),
                key_id: pick.key_id.clone(),
                detail: format!("first pick → {}", pick.label),
            });
            pool.record(false, Some(429), t0);
            steps.push(PoolSimulationStep {
                action: "fail".into(),
                key_id: pick.key_id.clone(),
                detail: "HTTP 429 → cooldown stage 1 (3s)".into(),
            });
        }

        if let Some(pick) = pool.pick_next(t0) {
            steps.push(PoolSimulationStep {
                action: "pick".into(),
                key_id: pick.key_id.clone(),
                detail: "failover to second key".into(),
            });
            pool.record(true, Some(200), t0);
            steps.push(PoolSimulationStep {
                action: "ok".into(),
                key_id: pick.key_id.clone(),
                detail: "success".into(),
            });
        }

        steps
    }
}

/// 从 state.json + Keychain 构建运行时密钥池（Phase 1 代理 failover）
pub fn build_runtime_pool(supplier_id: &str) -> Result<KeyPool, AppError> {
    let state = load_state()?;
    let mut keys = Vec::new();
    for meta in &state.pool_keys {
        if !meta.enabled {
            continue;
        }
        if get_api_key(supplier_id, &meta.id)?.is_some() {
            keys.push(PoolKey {
                id: meta.id.clone(),
                label: meta.label.clone(),
                weight: meta.weight.max(1),
                enabled: true,
            });
        }
    }
    if keys.is_empty() {
        if get_primary_key(supplier_id)?.is_some() {
            keys.push(PoolKey {
                id: "primary".into(),
                label: "主 Key".into(),
                weight: 1,
                enabled: true,
            });
        }
    }
    if keys.is_empty() {
        return Err(AppError::Message("Keychain 中无可用 API Key".into()));
    }
    Ok(KeyPool::new(keys, state.fail_threshold))
}

impl KeyRuntime {
    fn weight_score(&self) -> i64 {
        self.key.weight.max(1) as i64
    }

    fn available_at(&self, now: Instant) -> bool {
        self.key.enabled && self.cooling_until.map(|t| t <= now).unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cooldown_ladder_matches_beeapi() {
        assert_eq!(cooldown_seconds(1), 3);
        assert_eq!(cooldown_seconds(2), 10);
        assert_eq!(cooldown_seconds(5), 300);
    }

    #[test]
    fn simulate_failover_switches_key_after_429() {
        let steps = KeyPool::simulate_failover_demo();
        assert!(steps.len() >= 3);
        assert_eq!(steps[0].key_id, "k1");
        assert_eq!(steps[0].action, "pick");
        assert!(steps.iter().any(|s| s.action == "fail" && s.key_id == "k1"));
        assert!(steps.iter().any(|s| s.action == "pick" && s.key_id == "k2"));
    }

    #[test]
    fn snapshot_marks_cooling_key_unavailable() {
        let mut pool = KeyPool::new(
            vec![PoolKey {
                id: "k1".into(),
                label: "key-1".into(),
                weight: 1,
                enabled: true,
            }],
            1,
        );
        let t0 = Instant::now();
        let _ = pool.pick_next(t0);
        pool.record(false, Some(429), t0);
        let stats = pool.snapshot_stats(t0);
        assert_eq!(stats.len(), 1);
        assert!(!stats[0].available);
        assert_eq!(stats[0].cooldown_stage, 1);
    }
}
