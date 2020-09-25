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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_sender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_invite_subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smtp_reminder_subject: Option<String>,
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
                smtp_sender: Some("no_reply@localhost".to_string()),
                smtp_invite_subject: Some("Invitation for voting".to_string()),
                smtp_reminder_subject: Some("Invitation for voting (reminder)".to_string()),
            }
    }

    pub fn dump(&self) -> String {
        format!("baseURL: {}\ndisableLogin: {}\nenableAdmin: {}\nallowEditor: {}\nsmtp: {} with {},*** sender: {}\nMail subject: invite({}), remind({})",
            self.base_url, self.disable_login, self.enable_admin, self.allow_editor, self.smtp_server.as_ref().unwrap_or(&"sendmail".to_string()), self.smtp_username.as_ref().unwrap_or(&"anonymous".to_string()), self.smtp_sender.as_ref().unwrap_or(&"no_reply@<yourhost>".to_string()), self.smtp_invite_subject.as_ref().unwrap_or(&"We need you!".to_string()), self.smtp_reminder_subject.as_ref().unwrap_or(&"We still need you!".to_string())
        )
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
            cfg.smtp_invite_subject = Some("We need you!".to_string());
            cfg.smtp_reminder_subject = Some("We still need you!".to_string());
            serde_yaml::to_string(&cfg)?
        }
    };
    fs::write(path.unwrap_or("config.yml"), content).expect("Failed writing");
    return Ok(path.unwrap_or("config.yml").to_string());
}
