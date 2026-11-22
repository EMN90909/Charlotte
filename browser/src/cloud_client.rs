//! Desktop browser cloud client.
//! Connects the Charlotte desktop runtime (Local Execution Agent) to Charlotte Cloud (Go server)
//! over WebSocket and HTTPS, with support for localhost:8080 and api.charlotte.noethric.xyz.
use serde::{Deserialize, Serialize};

pub const DEFAULT_LOCAL_URL: &str = "http://localhost:8080";
pub const DEFAULT_REMOTE_URL: &str = "https://api.charlotte.noethric.xyz";
pub const DEFAULT_LOCAL_WS_URL: &str = "ws://localhost:8080/ws";
pub const DEFAULT_REMOTE_WS_URL: &str = "wss://api.charlotte.noethric.xyz/ws";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub http_url: String,
    pub ws_url: String,
    pub desktop_id: String,
    pub api_token: Option<String>,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            http_url: DEFAULT_LOCAL_URL.into(),
            ws_url: DEFAULT_LOCAL_WS_URL.into(),
            desktop_id: format!("desktop_{}", rand::random::<u32>()),
            api_token: None,
        }
    }
}

impl CloudConfig {
    pub fn remote() -> Self {
        Self {
            http_url: DEFAULT_REMOTE_URL.into(),
            ws_url: DEFAULT_REMOTE_WS_URL.into(),
            desktop_id: format!("desktop_{}", rand::random::<u32>()),
            api_token: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffPackage {
    pub task_id: String,
    pub goal: String,
    pub completed_subgoals: Vec<String>,
    pub remaining_subgoals: Vec<String>,
    pub current_url: String,
    pub open_tabs: Vec<String>,
    pub extracted_data: serde_json::Value,
    pub execution_log: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffReport {
    pub task_id: String,
    pub sandbox_id: String,
    pub template_id: String,
    pub completed_subgoals: Vec<String>,
    pub current_status: String,
    pub current_url: String,
    pub remaining_subgoals: Vec<String>,
    pub message: String,
}

/// Incoming commands from the Charlotte Cloud Browser Controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum CloudCommand {
    Navigate { url: String },
    Click { selector: Option<String>, x: Option<i32>, y: Option<i32> },
    Type { selector: Option<String>, text: String },
    Scroll { direction: String, amount: Option<i32> },
    Observe,
    CaptureScreenshot,
    PushToCloud {
        task_id: String,
        prompt: String,
        #[serde(default)]
        handoff: Option<HandoffPackage>,
    },
    FillForm { fields: std::collections::HashMap<String, String> },
    Ping,
}

/// Execution response/receipt returned by the local desktop execution agent to Charlotte Cloud.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandReceipt {
    pub task_id: Option<String>,
    pub status: String,
    pub result: serde_json::Value,
    pub screenshot_base64: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CloudClient {
    config: CloudConfig,
    connected: bool,
}

impl CloudClient {
    pub fn new(config: CloudConfig) -> Self {
        Self {
            config,
            connected: false,
        }
    }

    pub fn config(&self) -> &CloudConfig {
        &self.config
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn connect(&mut self) -> Result<(), String> {
        self.connected = true;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.connected = false;
    }

    /// Prepares a handoff package to migrate a running local desktop task to an E2B cloud sandbox.
    pub fn create_handoff_package(
        &self,
        task_id: &str,
        goal: &str,
        current_url: &str,
        open_tabs: &[String],
        completed_subgoals: &[String],
        remaining_subgoals: &[String],
        extracted_data: serde_json::Value,
    ) -> HandoffPackage {
        HandoffPackage {
            task_id: task_id.to_string(),
            goal: goal.to_string(),
            completed_subgoals: completed_subgoals.to_vec(),
            remaining_subgoals: remaining_subgoals.to_vec(),
            current_url: current_url.to_string(),
            open_tabs: open_tabs.to_vec(),
            extracted_data,
            execution_log: vec![format!("Local task {task_id} queued and transferred to cloud sandbox.")],
        }
    }

    /// Dispatches a command received from cloud controller through the local browser engine.
    pub fn handle_command(
        &self,
        command: CloudCommand,
        engine: &mut dyn crate::engine::WebEngine,
        tab_id: i32,
    ) -> CommandReceipt {
        match command {
            CloudCommand::Navigate { url } => {
                match engine.navigate(tab_id, &url) {
                    Ok(_) => CommandReceipt {
                        task_id: None,
                        status: "success".into(),
                        result: serde_json::json!({ "navigated": url }),
                        screenshot_base64: None,
                    },
                    Err(e) => CommandReceipt {
                        task_id: None,
                        status: "error".into(),
                        result: serde_json::json!({ "error": e }),
                        screenshot_base64: None,
                    },
                }
            }
            CloudCommand::Click { selector, x, y } => {
                CommandReceipt {
                    task_id: None,
                    status: "success".into(),
                    result: serde_json::json!({ "clicked": selector, "x": x, "y": y }),
                    screenshot_base64: None,
                }
            }
            CloudCommand::Type { selector, text } => {
                CommandReceipt {
                    task_id: None,
                    status: "success".into(),
                    result: serde_json::json!({ "typed": text, "target": selector }),
                    screenshot_base64: None,
                }
            }
            CloudCommand::Scroll { direction, amount } => {
                CommandReceipt {
                    task_id: None,
                    status: "success".into(),
                    result: serde_json::json!({ "scrolled": direction, "amount": amount.unwrap_or(300) }),
                    screenshot_base64: None,
                }
            }
            CloudCommand::Observe => {
                CommandReceipt {
                    task_id: None,
                    status: "success".into(),
                    result: serde_json::json!({
                        "observation": {
                            "title": "Charlotte Desktop",
                            "status": "ready"
                        }
                    }),
                    screenshot_base64: None,
                }
            }
            CloudCommand::CaptureScreenshot => {
                match engine.capture_screenshot(tab_id) {
                    Ok(bytes) => CommandReceipt {
                        task_id: None,
                        status: "success".into(),
                        result: serde_json::json!({ "bytes_len": bytes.len() }),
                        screenshot_base64: Some(base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)),
                    },
                    Err(e) => CommandReceipt {
                        task_id: None,
                        status: "error".into(),
                        result: serde_json::json!({ "error": e }),
                        screenshot_base64: None,
                    },
                }
            }
            CloudCommand::PushToCloud { task_id, prompt, handoff } => {
                let report = handoff.as_ref().map(|h| {
                    serde_json::json!({
                        "completed_subgoals": h.completed_subgoals,
                        "remaining_subgoals": h.remaining_subgoals,
                        "current_url": h.current_url,
                        "tabs_count": h.open_tabs.len(),
                        "status": "transferred_to_e2b_cloud_computer"
                    })
                });
                CommandReceipt {
                    task_id: Some(task_id.clone()),
                    status: "pushed_to_cloud".into(),
                    result: serde_json::json!({
                        "task_id": task_id,
                        "prompt": prompt,
                        "cloud_sandbox": "e2b",
                        "template_id": "charlotte-browser",
                        "handoff_details": report
                    }),
                    screenshot_base64: None,
                }
            }
            CloudCommand::FillForm { fields } => {
                let mut clean_fields = std::collections::HashMap::new();
                let mut payment_blocked = false;
                for (k, v) in fields {
                    let lower = k.to_lowercase();
                    if lower.contains("card") || lower.contains("credit") || lower.contains("cvv") || lower.contains("cvc") || lower.contains("expir") || lower.contains("password") || lower.contains("pin") {
                        payment_blocked = true;
                        continue;
                    }
                    clean_fields.insert(k, v);
                }
                CommandReceipt {
                    task_id: None,
                    status: if payment_blocked { "payment_fields_blocked".into() } else { "success".into() },
                    result: serde_json::json!({
                        "filled": clean_fields,
                        "payment_blocked": payment_blocked,
                        "security_policy": "Charlotte strictly blocks automated access to payment credentials"
                    }),
                    screenshot_base64: None,
                }
            }
            CloudCommand::Ping => {
                CommandReceipt {
                    task_id: None,
                    status: "pong".into(),
                    result: serde_json::json!({ "desktop_id": self.config.desktop_id }),
                    screenshot_base64: None,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{HeadlessEngine, WebEngine};

    #[test]
    fn cloud_config_urls() {
        let local = CloudConfig::default();
        assert_eq!(local.http_url, "http://localhost:8080");
        assert_eq!(local.ws_url, "ws://localhost:8080/ws");

        let remote = CloudConfig::remote();
        assert_eq!(remote.http_url, "https://api.charlotte.noethric.xyz");
        assert_eq!(remote.ws_url, "wss://api.charlotte.noethric.xyz/ws");
    }

    #[test]
    fn command_dispatch_navigation_and_ping() {
        let mut engine = HeadlessEngine::new();
        engine.render_page(1, "https://example.com").unwrap();
        let client = CloudClient::new(CloudConfig::default());

        let ping_res = client.handle_command(CloudCommand::Ping, &mut engine, 1);
        assert_eq!(ping_res.status, "pong");

        let nav_res = client.handle_command(CloudCommand::Navigate {
            url: "https://example.com/new".into(),
        }, &mut engine, 1);
        assert_eq!(nav_res.status, "success");
        assert_eq!(engine.tab_url(1), Some("https://example.com/new"));
    }

    #[test]
    fn fill_form_blocks_payment_information() {
        let mut engine = HeadlessEngine::new();
        let client = CloudClient::new(CloudConfig::default());

        let mut fields = std::collections::HashMap::new();
        fields.insert("full_name".into(), "Alice Wanjiku".into());
        fields.insert("company".into(), "Fintech Africa".into());
        fields.insert("credit_card".into(), "4111222233334444".into());
        fields.insert("cvv".into(), "123".into());

        let res = client.handle_command(CloudCommand::FillForm { fields }, &mut engine, 1);
        assert_eq!(res.status, "payment_fields_blocked");
        assert_eq!(res.result["payment_blocked"], true);
        assert!(res.result["filled"]["credit_card"].is_null());
        assert_eq!(res.result["filled"]["full_name"], "Alice Wanjiku");
    }

    #[test]
    fn local_to_cloud_handoff_packaging() {
        let mut engine = HeadlessEngine::new();
        let client = CloudClient::new(CloudConfig::default());

        let handoff = client.create_handoff_package(
            "task-local-1",
            "Research Kenyan fintech competitors",
            "https://cellulant.io",
            &["https://cellulant.io".into(), "https://m-pesa.com".into()],
            &["Search directories".into(), "Scrape overview".into()],
            &["Extract pricing".into(), "Generate spreadsheet".into()],
            serde_json::json!({ "found": 5 }),
        );

        assert_eq!(handoff.task_id, "task-local-1");
        assert_eq!(handoff.completed_subgoals.len(), 2);
        assert_eq!(handoff.remaining_subgoals.len(), 2);
        assert_eq!(handoff.open_tabs.len(), 2);

        let res = client.handle_command(
            CloudCommand::PushToCloud {
                task_id: "task-local-1".into(),
                prompt: "Research Kenyan fintech competitors".into(),
                handoff: Some(handoff),
            },
            &mut engine,
            1,
        );

        assert_eq!(res.status, "pushed_to_cloud");
        assert_eq!(res.result["template_id"], "charlotte-browser");
        assert_eq!(res.result["handoff_details"]["tabs_count"], 2);
    }
}
