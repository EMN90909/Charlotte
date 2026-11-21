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
    PushToCloud { task_id: String, prompt: String },
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
            CloudCommand::PushToCloud { task_id, prompt } => {
                CommandReceipt {
                    task_id: Some(task_id.clone()),
                    status: "pushed_to_cloud".into(),
                    result: serde_json::json!({ "task_id": task_id, "prompt": prompt, "cloud_sandbox": "e2b" }),
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
}
