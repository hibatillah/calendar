use serde::Deserialize;
use std::collections::HashMap;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Configuration {
    pub(super) enabled: bool,
    pub(super) project_token: String,
    pub(super) host: String,
}

impl Configuration {
    pub(super) fn load() -> Result<Self, &'static str> {
        let dotenv: HashMap<String, String> = match dotenvy::from_path_iter(".env") {
            Ok(entries) => entries
                .collect::<Result<_, _>>()
                .map_err(|_| "Invalid .env syntax · uploads off")?,
            Err(error) if error.not_found() => HashMap::new(),
            Err(_) => return Err("Cannot read .env · uploads off"),
        };
        let value = |name: &str| {
            std::env::var(name)
                .ok()
                .or_else(|| dotenv.get(name).cloned())
        };
        if let Some(configuration) = Self::from_values(value)? {
            return Ok(configuration);
        }
        let configuration = match std::fs::read_to_string(".local/telemetry.json") {
            Ok(contents) => serde_json::from_str::<Self>(&contents)
                .map_err(|_| "Invalid telemetry.json · uploads off")?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(_) => return Err("Cannot read telemetry.json · uploads off"),
        };
        configuration.validate()?;
        Ok(configuration)
    }

    pub(super) fn from_values(
        value: impl Fn(&str) -> Option<String>,
    ) -> Result<Option<Self>, &'static str> {
        let enabled = value("POSTHOG_ENABLED");
        let token = value("POSTHOG_PROJECT_TOKEN");
        let host = value("POSTHOG_HOST");
        if enabled.is_none() && token.is_none() && host.is_none() {
            return Ok(None);
        }
        let configuration = Self {
            enabled: match enabled.as_deref().unwrap_or("false") {
                "true" => true,
                "false" => false,
                _ => return Err("POSTHOG_ENABLED must be true or false · uploads off"),
            },
            project_token: token.unwrap_or_default(),
            host: host.unwrap_or_else(|| "https://us.i.posthog.com".into()),
        };
        configuration.validate()?;
        Ok(Some(configuration))
    }

    pub(super) fn validate(&self) -> Result<(), &'static str> {
        if self.enabled
            && (!self.project_token.starts_with("phc_")
                || self.project_token.len() < 12
                || !matches!(
                    self.host.as_str(),
                    "https://us.i.posthog.com" | "https://eu.i.posthog.com"
                ))
        {
            return Err("Invalid PostHog token or region · uploads off");
        }
        Ok(())
    }
}
