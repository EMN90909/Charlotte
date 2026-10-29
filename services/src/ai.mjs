import {randomInt} from "node:crypto";
import {PROVIDERS,providerKeys,enabledModels} from "./catalog.mjs";

export class GatewayError extends Error{
  constructor(status,message){super(message);this.status=status;}
}
function cleanMessages(history=[],system="",text=""){
  const messages=[];
  if(system)messages.push({role:"system",content:system});
  for(const turn of history.slice(-12)){
    if(turn&&["user","assistant"].includes(turn.role)&&typeof turn.text==="string")
      messages.push({role:turn.role,content:turn.text.slice(0,8000)});
  }
  messages.push({role:"user",content:text.slice(0,12000)});
  return messages;
}
function jpegInput(jpeg){
  if(jpeg==null)return null;
  if(typeof jpeg!=="string"||jpeg.length>1_600_000||jpeg.length<8||
    !/^[A-Za-z0-9+/]+={0,2}$/.test(jpeg))
    throw new GatewayError(400,"Invalid bounded JPEG base64");
  return jpeg;
}
export function buildUpstreamRequest(model,secret,input){
  const config=PROVIDERS[model.provider];
  if(!config||!secret)throw new GatewayError(503,"AI provider unavailable");
  const system=input.system||"";
  const messages=cleanMessages(input.history,system,input.text);
  const jpeg=jpegInput(input.image_jpeg_base64);
  const headers={"content-type":"application/json"};
  const maxTokens=Number.isSafeInteger(input.max_tokens)?
    Math.max(1,Math.min(input.max_tokens,2400)):1400;
  const setImageMessage=(entry)=>jpeg?{role:"user",content:[
    {type:"text",text:input.text},
    {type:"image_url",image_url:{url:`data:image/jpeg;base64,${jpeg}`,detail:"low"}}]}:entry;
  let body,url=config.url;
  if(config.api==="responses"){
    headers.authorization=`Bearer ${secret}`;
    const items=messages.filter(x=>x.role!=="system").map(m=>({role:m.role,content:
      m.role==="user"&&m===messages[messages.length-1]&&jpeg?
        [{type:"input_text",text:input.text},{type:"input_image",image_url:`data:image/jpeg;base64,${jpeg}`}]:
        [{type:m.role==="assistant"?"output_text":"input_text",text:m.content}]}));
    body={model:model.model,instructions:system,input:items,max_output_tokens:maxTokens};
  } else if(config.api==="anthropic"){
    headers["x-api-key"]=secret;headers["anthropic-version"]="2023-06-01";
    const turns=messages.filter(m=>m.role!=="system");
    if(jpeg)turns[turns.length-1]={role:"user",content:[
      {type:"text",text:input.text},{type:"image",source:{type:"base64",media_type:"image/jpeg",data:jpeg}}]};
    body={model:model.model,system,max_tokens:maxTokens,messages:turns};
  } else if(config.api==="gemini"){
    headers["x-goog-api-key"]=secret;
    url=`${config.url}/${encodeURIComponent(model.model)}:generateContent`;
    body={systemInstruction:{parts:[{text:system}]},contents:messages.filter(m=>m.role!=="system").map((m,i,arr)=>({
      role:m.role==="assistant"?"model":"user",
      parts:i===arr.length-1&&jpeg?[{text:input.text},{inlineData:{mimeType:"image/jpeg",data:jpeg}}]:[{text:m.content}]
    })),generationConfig:{maxOutputTokens:maxTokens}};
  } else if(config.api==="cohere"){
    if(jpeg)throw new GatewayError(422,"Vision not enabled for this model/API");
    headers.authorization=`Bearer ${secret}`;
    body={model:model.model,messages,max_tokens:maxTokens};
  } else {
    if(jpeg&&!["xai","openrouter","together","fireworks"].includes(model.provider))
      throw new GatewayError(422,"Vision not enabled for this model/API");
    headers.authorization=`Bearer ${secret}`;
    const turns=[...messages];
    if(jpeg)turns[turns.length-1]=setImageMessage(turns[turns.length-1]);
    body={model:model.model,messages:turns,stream:false,max_tokens:maxTokens};
  }
  return {url,headers,body};
}
function extractText(data,api){
  if(api==="responses"){
    const texts=(data.output||[]).flatMap(o=>(o.content||[])
      .filter(c=>c.type==="output_text"&&typeof c.text==="string")
      .map(c=>c.text));
    return texts.join("\n");
  }
  if(api==="anthropic")return (data.content||[]).filter(c=>c.type==="text").map(c=>c.text).join("\n");
  if(api==="gemini")return (data.candidates?.[0]?.content?.parts||[])
    .filter(x=>typeof x.text==="string").map(x=>x.text).join("\n");
  if(api==="cohere")return (data.message?.content||[]).filter(c=>c.type==="text").map(c=>c.text).join("\n");
  const content=data.choices?.[0]?.message?.content;
  return typeof content==="string"?content:
    Array.isArray(content)?content.filter(x=>x.type==="text").map(x=>x.text).join("\n"):"";
}
async function boundedJson(response,max=2_000_000){
  const reader=response.body?.getReader();
  if(!reader)throw new GatewayError(502,"Empty provider response");
  let size=0;const chunks=[];
  try{
    while(true){
      const {done,value}=await reader.read();if(done)break;
      size+=value.length;if(size>max)throw new GatewayError(502,"Provider response exceeded limit");
      chunks.push(value);
    }
  }finally{await reader.cancel().catch(()=>{});}
  try{return JSON.parse(Buffer.concat(chunks).toString("utf8"))}
  catch{throw new GatewayError(502,"Invalid provider JSON response");}
}
const allowedVision=new Set(["openai","anthropic","gemini","xai","openrouter","together","fireworks"]);
export async function generate(input,{env=process.env,fetchImpl=fetch}={}){
  if(typeof input!=="object"||input===null||Array.isArray(input))
    throw new GatewayError(400,"Invalid request object");
  const text=input.text;
  if(typeof text!=="string"||!text.trim()||text.length>12000)
    throw new GatewayError(400,"Text required (max 12000 chars)");
  if(input.system!==undefined&&(typeof input.system!=="string"||input.system.length>8000))
    throw new GatewayError(400,"System prompt too large");
  if(input.history!==undefined&&(!Array.isArray(input.history)||input.history.length>16))
    throw new GatewayError(400,"History too long");
  const models=enabledModels(env).filter(m=>!input.image_jpeg_base64||allowedVision.has(m.provider));
  if(!models.length)throw new GatewayError(503,"No configured AI model for this request");
  // "auto" must not randomly choose an expensive frontier model.
  // The operator controls the eligible auto pool; the fallback is a
  // cost-conscious ordered default. Randomize API KEYS, not the billable model.
  const preferred=(env.CLOUD_AUTO_MODELS||"").split(",").map(v=>v.trim()).filter(Boolean);
  const sensible=["openai/gpt-6-luna","gemini/gemini-3.5-flash-lite",
    "gemini/gemini-3.8-flash","groq/openai/gpt-oss-20b",
    "anthropic/claude-haiku-5-5","deepseek/deepseek-flash"];
  const selected=(preferred.length?preferred:sensible).map(id=>models.find(m=>m.id===id))
    .find(Boolean)||models[0];
  const model=input.model&&input.model!=="auto"?
    models.find(m=>m.id===input.model):selected;
  if(!model)throw new GatewayError(422,"Model not enabled or not available for this task");
  const keys=providerKeys(model.provider,env);
  const order=keys.map((_,i)=>i);
  // Cryptographically randomized pool order; at most two provider attempts.
  for(let i=order.length-1;i>0;i--){const j=randomInt(i+1);[order[i],order[j]]=[order[j],order[i]];}
  let lastStatus=503;
  for(let attempt=0;attempt<Math.min(order.length,2);attempt++){
    const key=keys[order[attempt]];
    const request=buildUpstreamRequest(model,key,input);
    const controller=new AbortController();
    const timeout=setTimeout(()=>controller.abort(),45000);
    try{
      const result=await fetchImpl(request.url,{
        method:"POST",headers:request.headers,body:JSON.stringify(request.body),
        signal:controller.signal,redirect:"error"
      });
      lastStatus=result.status;
      if(!result.ok){
        if([429,502,503,504].includes(result.status)&&attempt+1<Math.min(keys.length,2))continue;
        throw new GatewayError([429,502,503,504].includes(result.status)?503:502,
          "Configured upstream provider unavailable");
      }
      const body=await boundedJson(result);
      const answer=extractText(body,PROVIDERS[model.provider].api);
      if(!answer||answer.length>100_000)throw new GatewayError(502,"Unexpected AI provider response");
      return {text:answer,model:model.id,provider:model.provider};
    }catch(error){
      if(error instanceof GatewayError)throw error;
      lastStatus=503;
      if(attempt+1>=Math.min(keys.length,2))throw new GatewayError(503,"AI provider connection unavailable");
    }finally{clearTimeout(timeout);}
  }
  throw new GatewayError(lastStatus===429?429:503,"AI capacity is temporarily unavailable");
}
