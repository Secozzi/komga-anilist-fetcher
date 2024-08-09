use anyhow::Result;
use inquire::{required, Password, Text};
use serde::{Deserialize, Serialize};

const APP_NAME: &str = "komga-anilist-fetcher";
const CONFIG_NAME: &str = "config";

#[derive(Debug, Serialize, Deserialize)]
pub struct KomgaConfig {
    pub url: String,
    pub email: String,
    pub password: String,
}

impl Default for KomgaConfig {
    fn default() -> Self {
        Self {
            url: "".into(),
            email: "".into(),
            password: "".into(),
        }
    }
}

pub fn get_config() -> Result<KomgaConfig> {
    let cfg: KomgaConfig = confy::load(APP_NAME, CONFIG_NAME)?;
    if cfg.url.is_empty() || cfg.email.is_empty() || cfg.password.is_empty() {
        let new_cfg = generate_new_config()?;
        confy::store(APP_NAME, CONFIG_NAME, &new_cfg)?;
        return Ok(new_cfg);
    }

    Ok(cfg)
}

fn generate_new_config() -> Result<KomgaConfig> {
    let url = Text::new("Enter komga url:")
        .with_validator(required!())
        .prompt()?;
    let email = Text::new("Enter email address:")
        .with_validator(required!())
        .prompt()?;
    let password = Password::new("Enter password:")
        .without_confirmation()
        .prompt()?;

    Ok(KomgaConfig {
        url,
        email,
        password,
    })
}
