import {generateKeyPairSync} from "node:crypto";
import {writeFile,chmod,mkdir} from "node:fs/promises";
import {resolve,dirname} from "node:path";
// Run once on a trusted release-signing machine. Never commit/share the
// private key with the desktop client or an untrusted CI runner.
const target=resolve(process.argv[2]||"services/update-private.pem");
const pubPath=target.replace(/\.pem$/,"-public.pem");
await mkdir(dirname(target),{recursive:true});
const {privateKey,publicKey}=generateKeyPairSync("ed25519");
const secret=privateKey.export({format:"pem",type:"pkcs8"});
const publicPem=publicKey.export({format:"pem",type:"spki"});
await writeFile(target,secret,{mode:0o600,flag:"wx"});
await chmod(target,0o600);
await writeFile(pubPath,publicPem,{mode:0o644,flag:"wx"});
const der=publicKey.export({format:"der",type:"spki"});
console.log("Private signing key saved to a local 0600 file; store it in your production secret manager:");
console.log(target);
console.log("Compile Charlotte browser releases with this PINNED PUBLIC KEY only:");
console.log("CHARLOTTE_RELEASE_PUBKEY_HEX="+der.subarray(-32).toString("hex"));
console.log("Set service UPDATE_SIGNING_PRIVATE_KEY_PEM_BASE64 from the private PEM content.");
console.log("Never obtain the trusted public key dynamically from the update gateway.");
