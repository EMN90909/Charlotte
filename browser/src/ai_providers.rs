//! First-party BYOK and cloud AI model choices. This curated set is a
//! convenience, not a promise of account entitlement or permanent model life.
//! Users can type other valid model identifiers released by each provider.
#[derive(Debug,Clone,Copy)]
pub struct Provider {
    pub id:&'static str, pub name:&'static str,
    pub default_model:&'static str, pub models:&'static [&'static str],
    pub api_base:&'static str, pub kind:&'static str,
}
pub const PROVIDERS:&[Provider]=&[
    Provider{id:"openai",name:"OpenAI",default_model:"gpt-6-luna",models:&["gpt-6-luna","gpt-6.1-sol","gpt-6.1-terra","gpt-6-astra","gpt-5.6-sol"],api_base:"https://api.openai.com",kind:"openai"},
    Provider{id:"claude",name:"Anthropic Claude",default_model:"claude-sonnet-5-5",models:&["claude-sonnet-5-5","claude-haiku-5-5","claude-opus-5-5","claude-fable-5-1"],api_base:"https://api.anthropic.com",kind:"anthropic"},
    Provider{id:"gemini",name:"Google Gemini",default_model:"gemini-3.8-flash",models:&["gemini-3.8-flash","gemini-4-argon","gemini-3.6-flash","gemini-3.5-flash-lite"],api_base:"https://generativelanguage.googleapis.com",kind:"gemini"},
    Provider{id:"xai",name:"xAI Grok",default_model:"grok-4.7",models:&["grok-4.7","grok-4.6"],api_base:"https://api.x.ai/v1",kind:"compatible"},
    Provider{id:"mistral",name:"Mistral AI",default_model:"mistral-small-latest",models:&["mistral-small-latest","mistral-large-latest","mistral-medium-latest"],api_base:"https://api.mistral.ai/v1",kind:"compatible"},
    Provider{id:"deepseek",name:"DeepSeek",default_model:"deepseek-flash",models:&["deepseek-flash","deepseek-v4-pro"],api_base:"https://api.deepseek.com",kind:"compatible"},
    Provider{id:"groq",name:"Groq",default_model:"openai/gpt-oss-120b",models:&["openai/gpt-oss-120b","openai/gpt-oss-20b","llama-3.3-70b-versatile"],api_base:"https://api.groq.com/openai/v1",kind:"compatible"},
    Provider{id:"cohere",name:"Cohere",default_model:"command-a-03-2025",models:&["command-a-03-2025","command-a-plus-05-2026","command-r7b-12-2024"],api_base:"https://api.cohere.com",kind:"cohere"},
    Provider{id:"openrouter",name:"OpenRouter",default_model:"openai/gpt-oss-120b",models:&["openai/gpt-oss-120b"],api_base:"https://openrouter.ai/api/v1",kind:"compatible"},
    Provider{id:"together",name:"Together AI",default_model:"meta-llama/Llama-3.3-70B-Instruct-Turbo",models:&["meta-llama/Llama-3.3-70B-Instruct-Turbo"],api_base:"https://api.together.xyz/v1",kind:"compatible"},
    Provider{id:"fireworks",name:"Fireworks AI",default_model:"accounts/fireworks/models/llama-v3p3-70b-instruct",models:&["accounts/fireworks/models/llama-v3p3-70b-instruct"],api_base:"https://api.fireworks.ai/inference/v1",kind:"compatible"},
    Provider{id:"cerebras",name:"Cerebras",default_model:"gpt-oss-120b",models:&["gpt-oss-120b"],api_base:"https://api.cerebras.ai/v1",kind:"compatible"},
    Provider{id:"perplexity",name:"Perplexity Sonar",default_model:"sonar",models:&["sonar","sonar-pro"],api_base:"https://api.perplexity.ai",kind:"compatible"},
    Provider{id:"ollama",name:"Local Ollama",default_model:"llama3.2",models:&["llama3.2","llava"],api_base:"http://localhost:11434",kind:"ollama"},
    Provider{id:"cloud",name:"Charlotte Cloud (signed-in account)",default_model:"auto",models:&["auto"],api_base:"",kind:"cloud"},
];
pub fn get(id:&str)->Option<&'static Provider>{PROVIDERS.iter().find(|x|x.id==id)}
pub fn valid_model(model:&str)->bool{
    model.len()>0 && model.len()<=120 &&
    model.bytes().all(|c|c.is_ascii_alphanumeric()||matches!(c,b'.'|b'/'|b':'|b'-'|b'_'))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn catalog_covers_more_than_ten_api_providers_and_twenty_models(){
        assert!(PROVIDERS.len()>10);
        assert!(PROVIDERS.iter().map(|p|p.models.len()).sum::<usize>()>20);
        assert!(PROVIDERS.iter().all(|p|valid_model(p.default_model)));
    }
}
