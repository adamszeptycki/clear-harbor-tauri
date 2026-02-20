use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub api_key: Option<String>,
    pub language: String,
    pub mic_device_id: Option<String>,
    pub system_device_id: Option<String>,
    pub font_size: u32,
    pub theme: String,
    pub timestamps_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            api_key: None,
            language: "en".to_string(),
            mic_device_id: None,
            system_device_id: None,
            font_size: 14,
            theme: "system".to_string(),
            timestamps_enabled: true,
        }
    }
}
