//! Charlotte's local behavioral intelligence and user learning layer.
//! Tracks user workflows (frequent sites, button clicks, form fills, contacts, connectors)
//! strictly on device in local storage. Never records passwords, credit cards, or financial secrets.
//! Provides contextual memory to Charlotte conversational AI and autonomous local agents.

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn is_sensitive_credential_or_payment(field_name: &str, value: &str) -> bool {
    let lower_name = field_name.to_lowercase();
    let lower_val = value.to_lowercase();
    let sensitive_keywords = [
        "card", "credit", "cvv", "cvc", "expir", "password", "pin", "ssn",
        "secret", "token", "auth", "billing_card", "passcode", "security_code"
    ];

    for kw in &sensitive_keywords {
        if lower_name.contains(kw) || lower_val.contains(kw) {
            return true;
        }
    }

    // Heuristic: Credit card Luhn-like or 13-19 continuous digits
    let digits_only: String = value.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits_only.len() >= 13 && digits_only.len() <= 19 {
        return true;
    }

    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormInteraction {
    pub domain: String,
    pub form_id: String,
    pub field_name: String,
    pub field_type: String,
    pub sanitized_value: String,
    pub fill_count: u32,
    pub last_filled: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButtonInteraction {
    pub domain: String,
    pub button_text: String,
    pub context_action: String,
    pub click_count: u32,
    pub last_clicked: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteVisit {
    pub domain: String,
    pub visit_count: u32,
    pub total_duration_secs: u64,
    pub category: String,
    pub last_visited: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInteraction {
    pub recipient_email: String,
    pub domain: String,
    pub interaction_count: u32,
    pub last_contacted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorUsage {
    pub connector_name: String,
    pub tool_name: String,
    pub usage_count: u32,
    pub last_used: u64,
}

#[derive(Default)]
struct LearningStore {
    forms: Vec<FormInteraction>,
    buttons: Vec<ButtonInteraction>,
    sites: Vec<SiteVisit>,
    contacts: Vec<ContactInteraction>,
    connectors: Vec<ConnectorUsage>,
}

#[derive(Clone)]
pub struct UserLearningEngine {
    store: Arc<Mutex<LearningStore>>,
}

impl Default for UserLearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl UserLearningEngine {
    pub fn new() -> Self {
        Self {
            store: Arc::new(Mutex::new(LearningStore::default())),
        }
    }

    /// Records form field entry with strict payment and password redaction.
    /// Returns true if accepted and learned, false if rejected due to sensitive data.
    pub fn record_form_fill(
        &self,
        domain: &str,
        form_id: &str,
        field_name: &str,
        field_type: &str,
        value: &str,
    ) -> bool {
        if is_sensitive_credential_or_payment(field_name, value) || field_type == "password" {
            return false;
        }

        let mut store = self.store.lock().unwrap();
        let now = now_epoch_secs();

        if let Some(existing) = store.forms.iter_mut().find(|f| {
            f.domain == domain && f.field_name == field_name
        }) {
            existing.fill_count += 1;
            existing.last_filled = now;
            existing.sanitized_value = value.to_string();
        } else {
            store.forms.push(FormInteraction {
                domain: domain.to_string(),
                form_id: form_id.to_string(),
                field_name: field_name.to_string(),
                field_type: field_type.to_string(),
                sanitized_value: value.to_string(),
                fill_count: 1,
                last_filled: now,
            });
        }

        true
    }

    /// Records interactive button clicks and action intents.
    pub fn record_button_click(&self, domain: &str, button_text: &str, context_action: &str) {
        let mut store = self.store.lock().unwrap();
        let now = now_epoch_secs();

        if let Some(existing) = store.buttons.iter_mut().find(|b| {
            b.domain == domain && b.button_text == button_text
        }) {
            existing.click_count += 1;
            existing.last_clicked = now;
        } else {
            store.buttons.push(ButtonInteraction {
                domain: domain.to_string(),
                button_text: button_text.to_string(),
                context_action: context_action.to_string(),
                click_count: 1,
                last_clicked: now,
            });
        }
    }

    /// Records site visit frequency and category.
    pub fn record_site_visit(&self, domain: &str, duration_secs: u64, category: &str) {
        let mut store = self.store.lock().unwrap();
        let now = now_epoch_secs();

        if let Some(existing) = store.sites.iter_mut().find(|s| s.domain == domain) {
            existing.visit_count += 1;
            existing.total_duration_secs += duration_secs;
            existing.last_visited = now;
        } else {
            store.sites.push(SiteVisit {
                domain: domain.to_string(),
                visit_count: 1,
                total_duration_secs: duration_secs,
                category: category.to_string(),
                last_visited: now,
            });
        }
    }

    /// Records email contact interaction.
    pub fn record_contact_interaction(&self, recipient_email: &str) {
        let mut store = self.store.lock().unwrap();
        let now = now_epoch_secs();
        let domain = recipient_email.split('@').nth(1).unwrap_or("unknown");

        if let Some(existing) = store.contacts.iter_mut().find(|c| c.recipient_email == recipient_email) {
            existing.interaction_count += 1;
            existing.last_contacted = now;
        } else {
            store.contacts.push(ContactInteraction {
                recipient_email: recipient_email.to_string(),
                domain: domain.to_string(),
                interaction_count: 1,
                last_contacted: now,
            });
        }
    }

    /// Records connector tool usage.
    pub fn record_connector_usage(&self, connector_name: &str, tool_name: &str) {
        let mut store = self.store.lock().unwrap();
        let now = now_epoch_secs();

        if let Some(existing) = store.connectors.iter_mut().find(|c| {
            c.connector_name == connector_name && c.tool_name == tool_name
        }) {
            existing.usage_count += 1;
            existing.last_used = now;
        } else {
            store.connectors.push(ConnectorUsage {
                connector_name: connector_name.to_string(),
                tool_name: tool_name.to_string(),
                usage_count: 1,
                last_used: now,
            });
        }
    }

    /// Returns most frequently visited sites.
    pub fn get_top_sites(&self, limit: usize) -> Vec<SiteVisit> {
        let store = self.store.lock().unwrap();
        let mut sites = store.sites.clone();
        sites.sort_by(|a, b| b.visit_count.cmp(&a.visit_count));
        sites.truncate(limit);
        sites
    }

    /// Returns most frequent contacts/emails.
    pub fn get_top_contacts(&self, limit: usize) -> Vec<ContactInteraction> {
        let store = self.store.lock().unwrap();
        let mut contacts = store.contacts.clone();
        contacts.sort_by(|a, b| b.interaction_count.cmp(&a.interaction_count));
        contacts.truncate(limit);
        contacts
    }

    /// Returns most frequently used connectors.
    pub fn get_top_connectors(&self, limit: usize) -> Vec<ConnectorUsage> {
        let store = self.store.lock().unwrap();
        let mut connectors = store.connectors.clone();
        connectors.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));
        connectors.truncate(limit);
        connectors
    }

    /// Suggests previously learned value for a form field.
    pub fn get_form_autofill_suggestion(&self, domain: &str, field_name: &str) -> Option<String> {
        let store = self.store.lock().unwrap();
        store.forms.iter().find(|f| {
            f.domain == domain && f.field_name == field_name
        }).map(|f| f.sanitized_value.clone())
    }

    /// Synthesizes behavioral intelligence into a concise prompt context for Charlotte agent.
    pub fn generate_agent_context_summary(&self) -> String {
        let top_sites = self.get_top_sites(3);
        let top_contacts = self.get_top_contacts(3);
        let top_connectors = self.get_top_connectors(3);

        let mut lines = Vec::new();
        if !top_sites.is_empty() {
            let names: Vec<String> = top_sites.iter().map(|s| format!("{} ({}x)", s.domain, s.visit_count)).collect();
            lines.push(format!("Frequent Sites: {}", names.join(", ")));
        }
        if !top_contacts.is_empty() {
            let emails: Vec<String> = top_contacts.iter().map(|c| c.recipient_email.clone()).collect();
            lines.push(format!("Frequent Email Contacts: {}", emails.join(", ")));
        }
        if !top_connectors.is_empty() {
            let conns: Vec<String> = top_connectors.iter().map(|c| c.connector_name.clone()).collect();
            lines.push(format!("Active Connectors: {}", conns.join(", ")));
        }

        if lines.is_empty() {
            "No historical usage recorded yet.".to_string()
        } else {
            lines.join(" | ")
        }
    }

    /// Privacy purge: clears all locally learned behavioral data upon user request.
    pub fn clear_learning_data(&self) {
        let mut store = self.store.lock().unwrap();
        *store = LearningStore::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn learns_safe_form_fields_and_strictly_rejects_payment_cards() {
        let engine = UserLearningEngine::new();

        // 1. Safe fields are learned
        assert!(engine.record_form_fill("acme.com", "signup", "full_name", "text", "John Doe"));
        assert!(engine.record_form_fill("acme.com", "signup", "company", "text", "Noethric Tech"));

        // 2. Sensitive fields (passwords, credit cards, CVVs) are strictly rejected
        assert!(!engine.record_form_fill("checkout.com", "pay", "credit_card_number", "text", "4111222233334444"));
        assert!(!engine.record_form_fill("checkout.com", "pay", "cvv", "text", "123"));
        assert!(!engine.record_form_fill("auth.com", "login", "user_password", "password", "secret123"));

        // 3. Verify retrieval
        assert_eq!(engine.get_form_autofill_suggestion("acme.com", "company"), Some("Noethric Tech".into()));
        assert_eq!(engine.get_form_autofill_suggestion("checkout.com", "credit_card_number"), None);
    }

    #[test]
    fn tracks_sites_contacts_and_connectors_context() {
        let engine = UserLearningEngine::new();

        engine.record_site_visit("github.com", 120, "developer");
        engine.record_site_visit("github.com", 60, "developer");
        engine.record_site_visit("google.com", 30, "search");

        engine.record_contact_interaction("team@noethric.xyz");
        engine.record_contact_interaction("team@noethric.xyz");
        engine.record_contact_interaction("client@external.com");

        engine.record_connector_usage("Google Sheets", "create_row");
        engine.record_connector_usage("Google Drive", "upload_file");

        let summary = engine.generate_agent_context_summary();
        assert!(summary.contains("github.com"));
        assert!(summary.contains("team@noethric.xyz"));
        assert!(summary.contains("Google Sheets"));
    }
}
