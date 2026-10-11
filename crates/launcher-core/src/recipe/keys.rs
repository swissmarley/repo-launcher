//! Common keys projects ask for, with the page where a person can get one. The UI builds the
//! label from `service` ("OpenAI API key"), so no English lives here.

use serde::Serialize;

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyKind {
    ApiKey,
    Token,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnownKey {
    pub service: &'static str,
    pub kind: KeyKind,
    pub help_url: &'static str,
}

const fn k(service: &'static str, kind: KeyKind, help_url: &'static str) -> KnownKey {
    KnownKey {
        service,
        kind,
        help_url,
    }
}

use KeyKind::{ApiKey, Token};

const CATALOG: &[(&[&str], KnownKey)] = &[
    (
        &["OPENAI_API_KEY"],
        k("OpenAI", ApiKey, "https://platform.openai.com/api-keys"),
    ),
    (
        &["ANTHROPIC_API_KEY", "CLAUDE_API_KEY"],
        k(
            "Anthropic",
            ApiKey,
            "https://console.anthropic.com/settings/keys",
        ),
    ),
    (
        &[
            "HF_TOKEN",
            "HUGGINGFACEHUB_API_TOKEN",
            "HUGGING_FACE_HUB_TOKEN",
            "HUGGINGFACE_API_KEY",
            "HF_API_TOKEN",
            "HUGGINGFACE_TOKEN",
        ],
        k(
            "Hugging Face",
            Token,
            "https://huggingface.co/settings/tokens",
        ),
    ),
    (
        &[
            "GOOGLE_API_KEY",
            "GEMINI_API_KEY",
            "GOOGLE_GENERATIVE_AI_API_KEY",
        ],
        k(
            "Google Gemini",
            ApiKey,
            "https://aistudio.google.com/apikey",
        ),
    ),
    (
        &["GROQ_API_KEY"],
        k("Groq", ApiKey, "https://console.groq.com/keys"),
    ),
    (
        &["REPLICATE_API_TOKEN", "REPLICATE_API_KEY"],
        k(
            "Replicate",
            Token,
            "https://replicate.com/account/api-tokens",
        ),
    ),
    (
        &["MISTRAL_API_KEY"],
        k("Mistral AI", ApiKey, "https://console.mistral.ai/api-keys"),
    ),
    (
        &["COHERE_API_KEY", "CO_API_KEY"],
        k("Cohere", ApiKey, "https://dashboard.cohere.com/api-keys"),
    ),
    (
        &["TOGETHER_API_KEY"],
        k(
            "Together AI",
            ApiKey,
            "https://api.together.ai/settings/api-keys",
        ),
    ),
    (
        &["OPENROUTER_API_KEY"],
        k("OpenRouter", ApiKey, "https://openrouter.ai/keys"),
    ),
    (
        &["PERPLEXITY_API_KEY", "PPLX_API_KEY"],
        k(
            "Perplexity",
            ApiKey,
            "https://www.perplexity.ai/settings/api",
        ),
    ),
    (
        &["DEEPSEEK_API_KEY"],
        k("DeepSeek", ApiKey, "https://platform.deepseek.com/api_keys"),
    ),
    (&["XAI_API_KEY"], k("xAI", ApiKey, "https://console.x.ai")),
    (
        &["ELEVENLABS_API_KEY", "ELEVEN_API_KEY"],
        k(
            "ElevenLabs",
            ApiKey,
            "https://elevenlabs.io/app/settings/api-keys",
        ),
    ),
    (
        &["STABILITY_API_KEY"],
        k(
            "Stability AI",
            ApiKey,
            "https://platform.stability.ai/account/keys",
        ),
    ),
    (
        &["PINECONE_API_KEY"],
        k("Pinecone", ApiKey, "https://app.pinecone.io"),
    ),
    (
        &["TAVILY_API_KEY"],
        k("Tavily", ApiKey, "https://app.tavily.com"),
    ),
    (
        &["SERPAPI_API_KEY", "SERPAPI_KEY", "SERP_API_KEY"],
        k("SerpApi", ApiKey, "https://serpapi.com/manage-api-key"),
    ),
    (
        &["LANGCHAIN_API_KEY", "LANGSMITH_API_KEY"],
        k("LangSmith", ApiKey, "https://smith.langchain.com/settings"),
    ),
    (
        &["FIRECRAWL_API_KEY"],
        k(
            "Firecrawl",
            ApiKey,
            "https://www.firecrawl.dev/app/api-keys",
        ),
    ),
    (
        &["ASSEMBLYAI_API_KEY"],
        k(
            "AssemblyAI",
            ApiKey,
            "https://www.assemblyai.com/app/api-keys",
        ),
    ),
    (
        &["DEEPGRAM_API_KEY"],
        k("Deepgram", ApiKey, "https://console.deepgram.com"),
    ),
    (
        &["FAL_KEY", "FAL_API_KEY"],
        k("fal", ApiKey, "https://fal.ai/dashboard/keys"),
    ),
    (
        &["VOYAGE_API_KEY"],
        k("Voyage AI", ApiKey, "https://dash.voyageai.com/api-keys"),
    ),
    (
        &["JINA_API_KEY"],
        k("Jina AI", ApiKey, "https://jina.ai/api-dashboard"),
    ),
    (
        &["BRAVE_API_KEY", "BRAVE_SEARCH_API_KEY"],
        k(
            "Brave Search",
            ApiKey,
            "https://api-dashboard.search.brave.com/app/keys",
        ),
    ),
    (
        &["AZURE_OPENAI_API_KEY"],
        k("Azure OpenAI", ApiKey, "https://portal.azure.com"),
    ),
    (
        &["NOTION_API_KEY", "NOTION_TOKEN"],
        k("Notion", Token, "https://www.notion.so/my-integrations"),
    ),
    (
        &["TELEGRAM_BOT_TOKEN", "TELEGRAM_TOKEN"],
        k(
            "Telegram",
            Token,
            "https://core.telegram.org/bots/tutorial#obtain-your-bot-token",
        ),
    ),
    (
        &["DISCORD_TOKEN", "DISCORD_BOT_TOKEN"],
        k(
            "Discord",
            Token,
            "https://discord.com/developers/applications",
        ),
    ),
    (
        &["SLACK_BOT_TOKEN"],
        k("Slack", Token, "https://api.slack.com/apps"),
    ),
    (
        &["GITHUB_TOKEN", "GH_TOKEN"],
        k(
            "GitHub",
            Token,
            "https://github.com/settings/personal-access-tokens",
        ),
    ),
];

pub fn lookup(name: &str) -> Option<KnownKey> {
    CATALOG
        .iter()
        .find(|(names, _)| names.contains(&name))
        .map(|(_, key)| key.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_help_link_is_https() {
        for (names, key) in CATALOG {
            assert!(key.help_url.starts_with("https://"), "{names:?}");
        }
        assert_eq!(
            lookup("HUGGINGFACEHUB_API_TOKEN").unwrap().service,
            "Hugging Face"
        );
        assert!(lookup("SOMETHING_ELSE").is_none());
    }
}
