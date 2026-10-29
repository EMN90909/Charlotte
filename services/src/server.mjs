import http from "node:http";
import {fileURLToPath} from "node:url";
import {generate,GatewayError} from "./ai.mjs";
import {enabledModels,PROVIDERS} from "./catalog.mjs";
import {updateManifest} from "./updates.mjs";
const rate=new Map();
function json(res,status,data){
  const body=JSON.stringify(data);
  res.writeHead(status,{"content-type":"application/json; charset=utf-8",
    "content-length":Buffer.byteLength(body),"cache-control":"no-store",
    "x-content-type-options":"nosniff","referrer-policy":"no-referrer",
    "content-security-policy":"default-src 'none'"});
  res.end(body);
}
async function readJson(req,limit=2_300_000){
  const chunks=[];let bytes=0;
  for await(const chunk of req){
    bytes+=chunk.length;
    if(bytes>limit)throw new GatewayError(413,"Payload exceeded limit");
    chunks.push(chunk);
  }
  let body;
  try{body=JSON.parse(Buffer.concat(chunks).toString("utf8"));}
  catch{throw new GatewayError(400,"Invalid JSON");}
  if(!body||typeof body!=="object"||Array.isArray(body))
    throw new GatewayError(400,"Expected JSON object");
  return body;
}
export async function verifySupabaseToken(token,{env=process.env,fetchImpl=fetch}={}){
  if(!token||token.length>6000||!env.SUPABASE_URL||!env.SUPABASE_PUBLISHABLE_KEY)
    throw new GatewayError(401,"Valid account authentication required");
  const base=new URL(env.SUPABASE_URL);
  if(base.protocol!=="https:"&&!(base.protocol==="http:"&&["localhost","127.0.0.1"].includes(base.hostname)))
    throw new GatewayError(503,"Identity provider is misconfigured");
  const address=new URL("/auth/v1/user",base);
  const controller=new AbortController();
  const timeout=setTimeout(()=>controller.abort(),8000);
  try{
    const response=await fetchImpl(address,{headers:{
      apikey:env.SUPABASE_PUBLISHABLE_KEY,authorization:`Bearer ${token}`
    },signal:controller.signal,redirect:"error"});
    if(!response.ok)throw new GatewayError(401,"Session expired or invalid");
    const user=await response.json();
    if(typeof user.id!=="string"||!user.id||user.is_anonymous===true)
      throw new GatewayError(401,"Authenticated account required");
    return user.id;
  }catch(e){
    if(e instanceof GatewayError)throw e;
    throw new GatewayError(503,"Could not verify account with identity provider");
  }finally{clearTimeout(timeout);}
}
function withinQuota(user,env=process.env,now=Date.now()){
  const limit=Math.min(Math.max(Number(env.USER_REQUESTS_PER_HOUR||60),1),3000);
  const minute=60*60*1000,record=rate.get(user);
  if(!record||now-record.start>=minute){rate.set(user,{start:now,count:1});return true;}
  if(record.count>=limit)return false;
  record.count++;
  if(rate.size>10000){
    for(const [id,item] of rate)if(now-item.start>=minute)rate.delete(id);
  }
  return true;
}
export function createHandler({env=process.env,verifyUser=token=>verifySupabaseToken(token,{env}),
  generateFn=input=>generate(input,{env}),
  updateFn=query=>updateManifest(query,{env})}={}){
  return async(req,res)=>{
    try{
      const origin=`http://${req.headers.host||"localhost"}`;
      const path=new URL(req.url||"/",origin);
      if(req.method==="GET"&&path.pathname==="/health"){
        return json(res,200,{status:"ok",service:"charlotte-ai-gateway",version:1});
      }
      if(req.method==="GET"&&path.pathname==="/v1/updates/manifest"){
        const args=Object.fromEntries(path.searchParams);
        const result=await updateFn(args);
        return json(res,200,result);
      }
      const auth=req.headers.authorization||"";
      if(typeof auth!=="string"||!auth.startsWith("Bearer ")||auth.length>6100)
        throw new GatewayError(401,"Charlotte account sign-in required");
      const user=await verifyUser(auth.slice(7));
      if(!user)throw new GatewayError(401,"Invalid account");
      if(req.method==="GET"&&path.pathname==="/v1/models"){
        return json(res,200,{models:enabledModels(env).map(m=>({
          id:m.id,provider:m.provider,reference:m.reference
        })),providers:Object.keys(PROVIDERS)});
      }
      if(req.method==="POST"&&path.pathname==="/v1/ai/respond"){
        if(!withinQuota(user,env))throw new GatewayError(429,"Account request quota reached");
        if(!req.headers["content-type"]?.startsWith("application/json"))
          throw new GatewayError(415,"JSON content type required");
        const input=await readJson(req);
        const reply=await generateFn(input);
        return json(res,200,reply);
      }
      throw new GatewayError(404,"Unknown API route");
    }catch(error){
      const status=error instanceof GatewayError?error.status:503;
      // Never echo third-party response bodies, authentication tokens, user
      // prompts, environment variables, or internal stack traces to clients.
      const message=error instanceof GatewayError?error.message:"Gateway unavailable";
      return json(res,status,{error:message});
    }
  };
}
export function start(env=process.env){
  if(!env.SUPABASE_URL||!env.SUPABASE_PUBLISHABLE_KEY)
    throw Error("Set SUPABASE_URL and SUPABASE_PUBLISHABLE_KEY before starting Charlotte cloud services");
  const host=env.HOST||"127.0.0.1";
  const port=Number(env.PORT||8087);
  if(!Number.isSafeInteger(port)||port<1||port>65535)throw Error("Invalid PORT");
  const server=http.createServer(createHandler({env}));
  server.headersTimeout=10000;
  server.requestTimeout=60000;
  server.listen(port,host,()=>console.info(`Charlotte services listening on ${host}:${port}`));
  return server;
}
if(process.argv[1]&&fileURLToPath(import.meta.url)===process.argv[1]){
  start();
}
