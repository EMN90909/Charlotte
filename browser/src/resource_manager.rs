//! Tab sleeping and system resource management.
//! Monitors memory pressure and tab idle time, safely sleeping or discarding
//! background tabs while protecting active media playback, WebRTC, and active tabs.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabResourceState {
    Active,
    EcoOccluded,
    Suspended,
    Discarded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePolicy {
    pub idle_timeout_secs: u64,
    pub max_active_tabs: usize,
    pub auto_sleep_enabled: bool,
    pub memory_threshold_mb: usize,
}

impl Default for ResourcePolicy {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 900, // 15 minutes
            max_active_tabs: 8,
            auto_sleep_enabled: true,
            memory_threshold_mb: 2048, // 2 GB
        }
    }
}

#[derive(Debug, Clone)]
pub struct TabResourceMetrics {
    pub state: TabResourceState,
    pub estimated_memory_mb: usize,
    pub cpu_throttled: bool,
}

impl Default for TabResourceMetrics {
    fn default() -> Self {
        Self {
            state: TabResourceState::Active,
            estimated_memory_mb: 60,
            cpu_throttled: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct ResourceManager {
    policy: ResourcePolicy,
    tab_metrics: HashMap<i32, TabResourceMetrics>,
}

impl ResourceManager {
    pub fn new(policy: ResourcePolicy) -> Self {
        Self {
            policy,
            tab_metrics: HashMap::new(),
        }
    }

    pub fn policy(&self) -> &ResourcePolicy {
        &self.policy
    }

    pub fn set_policy(&mut self, policy: ResourcePolicy) {
        self.policy = policy;
    }

    pub fn tab_state(&self, tab_id: i32) -> TabResourceState {
        self.tab_metrics.get(&tab_id).map(|m| m.state).unwrap_or(TabResourceState::Active)
    }

    /// Enforces resource management rules across open tabs.
    /// Returns the list of tab IDs that were transitioned to sleeping/suspended.
    pub fn enforce(&mut self, tabs: &mut crate::tabs::Tabs) -> Vec<i32> {
        if !self.policy.auto_sleep_enabled {
            return Vec::new();
        }

        let _ = tabs.auto_sleep_idle_tabs(self.policy.idle_timeout_secs, self.policy.max_active_tabs);
        let active_id = tabs.active_id();
        let mut slept = Vec::new();

        for tab in tabs.all() {
            if Some(tab.id) == active_id || tab.media_playing {
                continue;
            }
            if tab.sleeping {
                slept.push(tab.id);
                let metric = self.tab_metrics.entry(tab.id).or_default();
                metric.state = TabResourceState::EcoOccluded;
                metric.cpu_throttled = true;
                metric.estimated_memory_mb = 24;
            }
        }

        // Active tab is always marked active
        if let Some(active_id) = active_id {
            let metric = self.tab_metrics.entry(active_id).or_default();
            metric.state = TabResourceState::Active;
            metric.cpu_throttled = false;
        }

        slept
    }

    /// Discard tab memory when under severe memory pressure, preserving state for instant restore.
    pub fn discard_tab(&mut self, tabs: &mut crate::tabs::Tabs, tab_id: i32) -> bool {
        if tabs.active_id() == Some(tab_id) {
            return false;
        }
        let Some(tab) = tabs.all().iter().find(|t| t.id == tab_id) else { return false };
        if tab.media_playing {
            return false;
        }
        if tabs.sleep_tab(tab_id) {
            let metric = self.tab_metrics.entry(tab_id).or_default();
            metric.state = TabResourceState::Discarded;
            metric.estimated_memory_mb = 2; // only metadata in RAM
            metric.cpu_throttled = true;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforce_policy_protects_active_and_media() {
        let mut tabs = crate::tabs::Tabs::default();
        tabs.open(1);
        tabs.open(2);
        tabs.open(3);
        tabs.activate(1);
        tabs.set_media_playing(2, true);

        let mut mgr = ResourceManager::new(ResourcePolicy {
            idle_timeout_secs: 0,
            max_active_tabs: 0,
            auto_sleep_enabled: true,
            memory_threshold_mb: 1024,
        });

        let slept = mgr.enforce(&mut tabs);
        assert!(slept.contains(&3));
        assert!(!slept.contains(&1)); // active tab is immune
        assert!(!slept.contains(&2)); // media playing tab is immune
        assert_eq!(mgr.tab_state(3), TabResourceState::EcoOccluded);
    }
}
