// Curated starter catalog: IDs are suggestions, never an entitlement claim.
// The /v1/models endpoint only publishes models configured with working env keys.
export const PROVIDERS = Object.freeze({
  openai:{env:"OPENAI",api:"responses",url:"https://api.openai.com/v1/responses",models:["gpt-6-luna","gpt-6.1-sol","gpt-6.1-terra","gpt-6-astra","gpt-5.6-sol"],reference:"https://developers.openai.com/api/docs/models"},
  anthropic:{env:"ANTHROPIC",api:"anthropic",url:"https://api.anthropic.com/v1/messages",models:["claude-sonnet-5-5","claude-haiku-5-5","claude-opus-5-5","claude-fable-5-1"],reference:"https://platform.claude.com/docs/en/models/overview"},
  gemini:{env:"GEMINI",api:"gemini",url:"https://generativelanguage.googleapis.com/v1beta/models",models:["gemini-4-argon","gemini-3.8-flash","gemini-3.6-flash","gemini-3.5-flash-lite"],reference:"https://ai.google.dev/gemini-api/docs/models"},
  xai:{env:"XAI",api:"openai",url:"https://api.x.ai/v1/chat/completions",models:["grok-4.7","grok-4.6"],reference:"https://docs.x.ai/developers/models"},
  mistral:{env:"MISTRAL",api:"openai",url:"https://api.mistral.ai/v1/chat/completions",models:["mistral-large-latest","mistral-small-latest","mistral-medium-latest"],reference:"https://docs.mistral.ai/models"},
  deepseek:{env:"DEEPSEEK",api:"openai",url:"https://api.deepseek.com/chat/completions",models:["deepseek-flash","deepseek-v4-pro"],reference:"https://api-docs.deepseek.com/en/"},
  groq:{env:"GROQ",api:"openai",url:"https://api.groq.com/openai/v1/chat/completions",models:["openai/gpt-oss-120b","openai/gpt-oss-20b","llama-3.3-70b-versatile"],reference:"https://console.groq.com/docs/models"},
  cohere:{env:"COHERE",api:"cohere",url:"https://api.cohere.com/v2/chat",models:["command-a-plus-05-2026","command-a-03-2025","command-r7b-12-2024"],reference:"https://docs.cohere.com/docs/models"},
  openrouter:{env:"OPENROUTER",api:"openai",url:"https://openrouter.ai/api/v1/chat/completions",models:["openai/gpt-oss-120b"],reference:"https://openrouter.ai/docs"},
  together:{env:"TOGETHER",api:"openai",url:"https://api.together.xyz/v1/chat/completions",models:["meta-llama/Llama-3.3-70B-Instruct-Turbo"],reference:"https://docs.together.ai/docs/serverless-models"},
  fireworks:{env:"FIREWORKS",api:"openai",url:"https://api.fireworks.ai/inference/v1/chat/completions",models:["accounts/fireworks/models/llama-v3p3-70b-instruct"],reference:"https://docs.fireworks.ai"},
  cerebras:{env:"CEREBRAS",api:"openai",url:"https://api.cerebras.ai/v1/chat/completions",models:["gpt-oss-120b"],reference:"https://inference-docs.cerebras.ai"},
  perplexity:{env:"PERPLEXITY",api:"openai",url:"https://api.perplexity.ai/chat/completions",models:["sonar","sonar-pro"],reference:"https://docs.perplexity.ai"}
});
const VALID_ID=/^[a-zA-Z0-9][a-zA-Z0-9._:/-]{0,119}$/;
export function providerKeys(provider,env=process.env){
  const name=PROVIDERS[provider]?.env;
  if(!name)return [];
  const values=[env[`${name}_API_KEYS`]||"",env[`${name}_API_KEY`]||"",
    ...Array.from({length:8},(_,i)=>env[`${name}_API_KEY_${i+1}`]||"")];
  return [...new Set(values.flatMap(v=>v.split(",")).map(v=>v.trim()).filter(v=>v.length>10&&v.length<4096))];
}
export function enabledModels(env=process.env){
  const explicit=(env.CLOUD_ENABLED_MODELS||"").split(",").map(v=>v.trim()).filter(Boolean);
  const all=Object.entries(PROVIDERS).flatMap(([provider,cfg])=>
    cfg.models.map(id=>({id:`${provider}/${id}`,provider,model:id,reference:cfg.reference})));
  const allowed=new Set(explicit.length?explicit:all.map(x=>x.id));
  return all.filter(x=>allowed.has(x.id)&&providerKeys(x.provider,env).length>0);
}
export function resolveModel(id,env=process.env){
  const models=enabledModels(env);
  if(id==="auto"||!id)return models.length?models[0]:null;
  if(typeof id!=="string"||!VALID_ID.test(id))return null;
  return models.find(x=>x.id===id)||null;
}
