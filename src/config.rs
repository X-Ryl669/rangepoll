extern crate glob; 

use std::fs;
use crate::rp_error::RPError;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub disable_login: bool,
    pub enable_admin: bool,
    pub allow_editor: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_port: Option<u16>, 
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_password: Option<String>,
}

impl Config {
    pub fn new() -> Config
    {
        Config { 
                base_url: "http://localhost".to_string(), 
                disable_login: false,
                enable_admin: false,
                allow_editor: true,
                smtp_server: Some("sendmail".to_string()),
                smtp_port: None,
                smtp_username: None,
                smtp_password: None,
            }
    }
}

pub fn get_config(path: Option<&str>) -> Result<Config, RPError> {
    let content = fs::read_to_string(path.unwrap_or("config.yml"))?;
    let config = serde_yaml::from_str(&content)?;
    return Ok(config);
}

pub fn save_config(cfg: Option<&Config>, path: Option<&str>) -> Result<String, RPError> {
    let content = match cfg {
        Some(v) => serde_yaml::to_string(v)?,
        None    => { 
            let mut cfg = Config::new(); 
            cfg.smtp_server = Some("sendmail_or_your.smtp.server".to_string());
            cfg.smtp_username = Some("user.name".to_string());
            cfg.smtp_port = Some(25);
            cfg.smtp_password = Some("super_secret".to_string());
            serde_yaml::to_string(&cfg)?
        }
    };
    fs::write(path.unwrap_or("config.yml"), content).expect("Failed writing");
    return Ok(path.unwrap_or("config.yml").to_string());
}
