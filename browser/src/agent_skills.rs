//! Charlotte's Agent Skills registry and management.
//! Allows defining, saving, editing, and matching specialized skills for Charlotte agent.
//! Each skill defines prompt instructions, trigger keywords, and allowed tools.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

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
    pub fn new() -> Self {
        let registry = Self {
            skills: Arc::new(Mutex::new(Vec::new())),
        };

        // Seed default built-in skills
        let now = now_epoch_secs();
        registry.register_skill(AgentSkill {
            id: "skill-fintech".into(),
            name: "Fintech & Market Intelligence".into(),
            description: "Researches fintech startups, compares pricing models and payment APIs".into(),
            trigger_keywords: vec!["fintech".into(), "pricing".into(), "founders".into(), "market".into()],
            system_instructions: "Search company directories, navigate to official sites, extract founders and products, and tabulate findings.".into(),
            allowed_tools: vec!["browser:navigate".into(), "browser:extract".into(), "sheets:create_spreadsheet".into()],
            enabled: true,
            created_at: now,
        });

        registry.register_skill(AgentSkill {
            id: "skill-spreadsheet".into(),
            name: "Spreadsheet & Data Exporter".into(),
            description: "Transforms extracted unstructured web research into structured Google Sheets or CSV rows".into(),
            trigger_keywords: vec!["spreadsheet".into(), "sheet".into(), "csv".into(), "excel".into()],
            system_instructions: "Format extracted rows into standard columns, verify cell values, and export clean datasets.".into(),
            allowed_tools: vec!["sheets:create_spreadsheet".into(), "drive:upload_file".into()],
            enabled: true,
            created_at: now,
        });

        registry.register_skill(AgentSkill {
            id: "skill-summarizer".into(),
            name: "Deep Web Summarizer & Fact Extractor".into(),
            description: "Reads multi-tab articles, removes fluff, and provides structured bulleted summaries".into(),
            trigger_keywords: vec!["summarize".into(), "summary".into(), "article".into(), "research".into()],
            system_instructions: "Extract primary claims, key metrics, and references from the active web pages without hallucinations.".into(),
            allowed_tools: vec!["browser:observe".into(), "browser:extract".into()],
            enabled: true,
            created_at: now,
        });

        registry
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

    /// Matches a user goal or prompt against enabled skills by keywords.
    pub fn match_skill(&self, query: &str) -> Option<AgentSkill> {
        let lower = query.to_lowercase();
        let skills = self.skills.lock().unwrap();

        for skill in skills.iter() {
            if !skill.enabled {
                continue;
            }
            for kw in &skill.trigger_keywords {
                if lower.contains(kw) {
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
    fn skills_registry_crud_and_matching() {
        let reg = SkillRegistry::new();

        // 1. Check built-ins
        let skills = reg.list_skills();
        assert!(skills.len() >= 3);

        // 2. Match skill
        let matched = reg.match_skill("Please research top 20 Kenyan fintech companies");
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().id, "skill-fintech");

        // 3. Register custom skill
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

        assert_eq!(reg.get_skill("skill-custom-seo").unwrap().name, "SEO Analyzer");
        assert!(reg.match_skill("Perform SEO audit").is_some());

        // 4. Toggle skill
        reg.toggle_skill("skill-custom-seo", false);
        assert!(reg.match_skill("Perform SEO audit").is_none());

        // 5. Delete skill
        assert!(reg.delete_skill("skill-custom-seo"));
        assert!(reg.get_skill("skill-custom-seo").is_none());
    }
}
