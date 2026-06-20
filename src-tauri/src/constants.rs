pub mod draft_status {
    pub const PENDING: &str = "pending";
    pub const POSTED: &str = "posted";
    #[allow(dead_code)]
    pub const SKIPPED: &str = "skipped";
}

pub mod settings {
    pub const XAI_API_KEY: &str = "xai_api_key";
    pub const GROK_MODEL: &str = "grok_model";
    pub const X_CONSUMER_KEY: &str = "x_consumer_key";
    pub const X_CONSUMER_SECRET: &str = "x_consumer_secret";
    pub const X_ACCESS_TOKEN: &str = "x_access_token";
    pub const X_ACCESS_TOKEN_SECRET: &str = "x_access_token_secret";
}

pub const DEFAULT_GROK_MODEL: &str = "grok-4.3";
pub const DEFAULT_DRAFT_COUNT: u32 = 3;
pub const MAX_DRAFT_COUNT: u32 = 10;
pub const RESEARCH_SOURCE_LIMIT: usize = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DraftStyle {
    #[default]
    Insight,
    Informative,
    Funny,
    Witty,
    Meme,
}

impl DraftStyle {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_lowercase().as_str() {
            "informative" => Self::Informative,
            "funny" => Self::Funny,
            "witty" => Self::Witty,
            "meme" => Self::Meme,
            _ => Self::Insight,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Insight => "insight",
            Self::Informative => "informative",
            Self::Funny => "funny",
            Self::Witty => "witty",
            Self::Meme => "meme",
        }
    }
}