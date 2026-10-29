import test from "node:test";
import assert from "node:assert/strict";
import {generate,GatewayError} from "../src/ai.mjs";
import {enabledModels,providerKeys,PROVIDERS} from "../src/catalog.mjs";
import {createHandler} from "../src/server.mjs";
import {createServer} from "node:http";
import {generateKeyPairSync,createPrivateKey} from "node:crypto";
import {updateManifest,verifySignedUpdate,validateUpdate} from "../src/updates.mjs";

const KEY_1="test-key-charlotte-111111";
const KEY_2="test-key-charlotte-222222";

test("catalog has 10+ providers, 20+ potential chat model names",()=>{
  assert.ok(Object.keys(PROVIDERS).length>=10);
  assert.ok(Object.values(PROVIDERS).flatMap(p=>p.models).length>20);
});
test("API key pools are server side, de-duplicated and never in model catalog",()=>{
  const env={OPENAI_API_KEY_1:KEY_1,OPENAI_API_KEY_2:KEY_2,OPENAI_API_KEY:KEY_1};
  assert.deepEqual(providerKeys("openai",env).sort(),[KEY_1,KEY_2].sort());
  const models=enabledModels(env);
  assert.ok(models.length>=4);
  assert.ok(!JSON.stringify(models).includes(KEY_1));
});
test("cloud keys are randomized and never sent to the browser",async()=>{
  const used=[];
  const env={GROQ_API_KEY_1:KEY_1,GROQ_API_KEY_2:KEY_2,
    CLOUD_ENABLED_MODELS:"groq/openai/gpt-oss-120b"};
  const fetchImpl=async(url,options)=>{
    used.push({url,headers:options.headers,body:JSON.parse(options.body)});
    return new Response(JSON.stringify({choices:[{message:{content:"Cloud answer"}}]}),{
      status:200,headers:{"content-type":"application/json"}
    });
  };
  for(let i=0;i<8;i++){
    const answer=await generate({model:"groq/openai/gpt-oss-120b",
      system:"Be precise",history:[],text:"Hello"},{env,fetchImpl});
    assert.equal(answer.text,"Cloud answer");
    assert.equal(answer.provider,"groq");
    assert.ok(!JSON.stringify(answer).includes(KEY_1));
  }
  assert.ok(used.every(c=>[KEY_1,KEY_2].some(k=>c.headers.authorization===`Bearer ${k}`)));
});
test("non-allowlisted models and unsupported URLs are rejected",async()=>{
  await assert.rejects(()=>generate({model:"evil-custom",text:"Hi"},{
    env:{OPENAI_API_KEY:KEY_1},fetchImpl:async()=>{throw Error("must not dispatch")}}),
    error=>error instanceof GatewayError&&error.status===422);
  assert.throws(()=>validateUpdate({version:"0.4.1",channel:"stable",
    platform:"windows",arch:"x64",url:"http://evil.example/pkg",
    sha256:"a".repeat(64),size:100},{UPDATE_ASSET_HOST:"evil.example"}));
});
test("cloud route requires account auth before model invocation",async()=>{
  let called=0;
  const handler=createHandler({
    env:{OPENAI_API_KEY:KEY_1,USER_REQUESTS_PER_HOUR:"2"},
    verifyUser:async token=>{if(token!=="signed-user-token")throw new GatewayError(401,"Unauthorized");return "user-123";},
    generateFn:async input=>{called++;return {text:"Done",model:input.model,provider:"openai"};},
    updateFn:async()=>({available:false})
  });
  const server=createServer(handler);
  await new Promise(resolve=>server.listen(0,"127.0.0.1",resolve));
  const base=`http://127.0.0.1:${server.address().port}`;
  try{
    const without=await fetch(base+"/v1/ai/respond",{
      method:"POST",headers:{"content-type":"application/json"},body:'{"text":"Hello"}'});
    assert.equal(without.status,401);assert.equal(called,0);
    const accepted=await fetch(base+"/v1/ai/respond",{
      method:"POST",headers:{"authorization":"Bearer signed-user-token","content-type":"application/json"},
      body:JSON.stringify({text:"Hello",model:"auto"})});
    assert.equal(accepted.status,200);assert.equal(called,1);
    const wrong=await fetch(base+"/v1/ai/respond",{
      method:"POST",headers:{"authorization":"Bearer wrong-token","content-type":"application/json"},
      body:'{"text":"Hello"}'});
    assert.equal(wrong.status,401);assert.equal(called,1);
  }finally{await new Promise(resolve=>server.close(resolve));}
});
test("update catalog is signed with Ed25519, and rejects tampering",async()=>{
  const {privateKey,publicKey}=generateKeyPairSync("ed25519");
  const key=privateKey.export({format:"pem",type:"pkcs8"}).toString();
  const item={version:"0.4.2",channel:"stable",platform:"linux",arch:"x64",
    url:"https://download.example.org/Charlotte-0.4.2.tar.gz",sha256:"a".repeat(64),size:2048};
  const env={UPDATE_SIGNING_PRIVATE_KEY_PEM_BASE64:Buffer.from(key).toString("base64"),
    UPDATE_MANIFEST_FILE:"/fake/releases.json",UPDATE_ASSET_HOST:"download.example.org"};
  const result=await updateManifest({channel:"stable",platform:"linux",arch:"x64",version:"0.4.0"},
    {env,read:async()=>JSON.stringify([item])});
  assert.equal(result.available,true);
  assert.ok(verifySignedUpdate(result,publicKey.export({format:"pem",type:"spki"}).toString()));
  assert.equal(verifySignedUpdate({...result,url:"https://download.example.org/evil.tar"},
    publicKey.export({format:"pem",type:"spki"}).toString()),false);
  assert.deepEqual(await updateManifest({channel:"stable",platform:"linux",arch:"x64",version:"0.4.2"},
    {env,read:async()=>JSON.stringify([item])}),{available:false});
});
