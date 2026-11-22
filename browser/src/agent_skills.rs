//! Charlotte's Agent Skills registry and management.
//! Allows users to define, save, toggle, delete, and match custom specialized skills for the agent.
//! Starts clean with zero forced/hardcoded built-in skills.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub trigger_keywords: Vec<String>,
    pub system_instructions: String,
    pub allowed_tools: Vec<String>,
    pub enabled: bool,
    pub created_at: u64,
}

#[derive(Clone)]
pub struct SkillRegistry {
    skills: Arc<Mutex<Vec<AgentSkill>>>,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    /// Creates a clean, empty skill registry with no mandatory hardcoded skills.
    pub fn new() -> Self {
        Self {
            skills: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn register_skill(&self, skill: AgentSkill) {
        let mut skills = self.skills.lock().unwrap();
        if let Some(pos) = skills.iter().position(|s| s.id == skill.id) {
            skills[pos] = skill;
        } else {
            skills.push(skill);
        }
    }

    pub fn list_skills(&self) -> Vec<AgentSkill> {
        let skills = self.skills.lock().unwrap();
        skills.clone()
    }

    pub fn get_skill(&self, id: &str) -> Option<AgentSkill> {
        let skills = self.skills.lock().unwrap();
        skills.iter().find(|s| s.id == id).cloned()
    }

    pub fn delete_skill(&self, id: &str) -> bool {
        let mut skills = self.skills.lock().unwrap();
        if let Some(pos) = skills.iter().position(|s| s.id == id) {
            skills.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn toggle_skill(&self, id: &str, enabled: bool) -> bool {
        let mut skills = self.skills.lock().unwrap();
        if let Some(skill) = skills.iter_mut().find(|s| s.id == id) {
            skill.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Matches a user goal or prompt against enabled user-defined skills by keywords.
    pub fn match_skill(&self, query: &str) -> Option<AgentSkill> {
        let lower = query.to_lowercase();
        let skills = self.skills.lock().unwrap();

        for skill in skills.iter() {
            if !skill.enabled {
                continue;
            }
            for kw in &skill.trigger_keywords {
                let kw_lower = kw.trim().to_lowercase();
                if !kw_lower.is_empty() && lower.contains(&kw_lower) {
                    return Some(skill.clone());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skills_registry_starts_empty_and_supports_user_skills() {
        let reg = SkillRegistry::new();

        // 1. Starts clean with zero forced built-in skills
        assert!(reg.list_skills().is_empty());
        assert!(reg.match_skill("research fintech spreadsheet summarize").is_none());

        // 2. Register user custom skill
        let custom = AgentSkill {
            id: "skill-custom-seo".into(),
            name: "SEO Analyzer".into(),
            description: "Audits page headings, meta tags, and alt text".into(),
            trigger_keywords: vec!["seo".into(), "audit".into()],
            system_instructions: "Extract headings and title tags from DOM.".into(),
            allowed_tools: vec!["browser:observe".into()],
            enabled: true,
            created_at: 1000,
        };
        reg.register_skill(custom);

        assert_eq!(reg.list_skills().len(), 1);
        assert_eq!(reg.get_skill("skill-custom-seo").unwrap().name, "SEO Analyzer");
        assert!(reg.match_skill("Perform SEO audit").is_some());

        // 3. Toggle skill
        reg.toggle_skill("skill-custom-seo", false);
        assert!(reg.match_skill("Perform SEO audit").is_none());

        // 4. Delete skill
        assert!(reg.delete_skill("skill-custom-seo"));
        assert!(reg.get_skill("skill-custom-seo").is_none());
    }
}
