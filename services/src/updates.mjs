import {createPrivateKey,createPublicKey,sign,verify} from "node:crypto";
import {readFile} from "node:fs/promises";
import {resolve} from "node:path";
const CHANNELS=new Set(["stable","beta"]);
const PLATFORMS=new Set(["windows","linux","macos"]);
const ARCHES=new Set(["x64","arm64"]);
const VERSION=/^\d{1,4}\.\d{1,4}\.\d{1,4}(?:-[a-zA-Z0-9.-]{1,24})?$/;
const HEX=/^[a-f0-9]{64}$/i;
export function canonicalUpdate(item){
  return [item.version,item.channel,item.platform,item.arch,item.url,
    item.sha256.toLowerCase(),String(item.size)].join("\n")+"\n";
}
export function validateUpdate(item,env=process.env){
  if(!item||!VERSION.test(item.version)||!CHANNELS.has(item.channel)||
    !PLATFORMS.has(item.platform)||!ARCHES.has(item.arch)||
    !HEX.test(item.sha256)||!Number.isSafeInteger(item.size)||
    item.size<1||item.size>2_500_000_000)
    throw Error("Invalid update manifest item");
  const url=new URL(item.url);
  const allowed=(env.UPDATE_ASSET_HOST||"").toLowerCase();
  if(url.protocol!=="https:"||!allowed||url.hostname.toLowerCase()!==allowed||
    url.username||url.password||url.hash||url.search)
    throw Error("Update URL must be HTTPS on configured release asset host");
  return {version:item.version,channel:item.channel,platform:item.platform,arch:item.arch,
    url:url.toString(),sha256:item.sha256.toLowerCase(),size:item.size};
}
function semver(version){return version.split(/[.-]/).slice(0,3).map(Number);}
function newer(a,b){
  const x=semver(a),y=semver(b);
  for(let i=0;i<3;i++){if(x[i]!==y[i])return x[i]>y[i];}
  return a!==b&&!a.includes("-")&&b.includes("-");
}
export async function updateManifest(query,{env=process.env,read=readFile}={}){
  const {channel,platform,arch,version}=query;
  if(!CHANNELS.has(channel)||!PLATFORMS.has(platform)||!ARCHES.has(arch)||
     !VERSION.test(version))throw Error("Invalid update platform or version");
  const keyData=env.UPDATE_SIGNING_PRIVATE_KEY_PEM_BASE64;
  const file=env.UPDATE_MANIFEST_FILE;
  if(!keyData||!file)return {available:false};
  // This is a server-owned deployment file, never user-supplied path.
  const data=await read(resolve(file),"utf8");
  if(data.length>256000)throw Error("Update catalog too large");
  const items=JSON.parse(data);
  if(!Array.isArray(items)||items.length>100)throw Error("Invalid update catalog");
  const matching=items.map(x=>validateUpdate(x,env)).filter(x=>
    x.channel===channel&&x.platform===platform&&x.arch===arch&&newer(x.version,version));
  matching.sort((a,b)=>newer(a.version,b.version)?-1:1);
  if(!matching.length)return {available:false};
  const item=matching[0];
  const privateKey=createPrivateKey(Buffer.from(keyData,"base64").toString("utf8"));
  if(privateKey.asymmetricKeyType!=="ed25519")throw Error("Update signing key is not Ed25519");
  const signature=sign(null,Buffer.from(canonicalUpdate(item)),privateKey).toString("base64");
  return {available:true,...item,signature,algorithm:"Ed25519"};
}
export function verifySignedUpdate(item,publicPem){
  const publicKey=createPublicKey(publicPem);
  return publicKey.asymmetricKeyType==="ed25519"&&
    verify(null,Buffer.from(canonicalUpdate(item)),publicKey,
      Buffer.from(item.signature,"base64"));
}
