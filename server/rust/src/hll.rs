use std::{fs,io::{self,Write},path::Path};
const N:usize=1<<18;
const MAGIC:&[u8]=b"charlotte-hll-v2\n";
pub struct Hll { registers:Vec<u8> }
impl Hll {
 pub fn new()->Self {Self{registers:vec![0;N]}}
 pub fn load(path:&str)->io::Result<Self> {
   if !Path::new(path).exists(){return Ok(Self::new())}
   let bytes=fs::read(path)?;
   if bytes.len()!=MAGIC.len()+N || !bytes.starts_with(MAGIC) {return Err(io::Error::new(io::ErrorKind::InvalidData,"invalid Charlotte HLL v2 file"))}
   Ok(Self{registers:bytes[MAGIC.len()..].to_vec()})
 }
 pub fn add(&mut self, s:&str){
   let mut x:u64=0xcbf29ce484222325;
   for b in s.as_bytes(){x^=*b as u64;x=x.wrapping_mul(0x100000001b3);}
   x^=x>>33;x=x.wrapping_mul(0xff51afd7ed558ccd);x^=x>>33;
   x=x.wrapping_mul(0xc4ceb9fe1a85ec53);x^=x>>33;
   let idx=x as usize & (N-1);
   let rank=((x>>18).leading_zeros()-18+1) as u8;
   self.registers[idx]=self.registers[idx].max(rank);
 }
 pub fn estimate(&self)->u64 {
   let mut sum=0.0;let mut zeros=0usize;
   for r in &self.registers {sum+=2f64.powi(-(*r as i32));if *r==0{zeros+=1;}}
   let m=N as f64;
   let raw=(0.7213/(1.0+1.079/m))*m*m/sum;
   if raw<=2.5*m && zeros>0 {(m*(m/zeros as f64).ln()).round() as u64} else {raw.round() as u64}
 }
 pub fn save(&self,path:&str)->io::Result<()> {
   if let Some(parent)=Path::new(path).parent(){fs::create_dir_all(parent)?;}
   let tmp=format!("{path}.tmp");
   let mut f=fs::File::create(&tmp)?;
   f.write_all(MAGIC)?;f.write_all(&self.registers)?;f.sync_all()?;drop(f);
   fs::rename(tmp,path)
 }
}
pub struct Metrics{pub rooms:Hll,pub users:Hll,room_path:String,user_path:String}
impl Metrics{
 pub fn load(room_path:String,user_path:String)->io::Result<Self>{
   Ok(Self{rooms:Hll::load(&room_path)?,users:Hll::load(&user_path)?,room_path,user_path})
 }
 pub fn add_room(&mut self, room:&str){self.rooms.add(room);}
 pub fn add_user(&mut self, user:&str){self.users.add(user);}
 pub fn snapshot(&self)->(u64,u64){(self.rooms.estimate(),self.users.estimate())}
 pub fn save(&self)->io::Result<()>{self.rooms.save(&self.room_path)?;self.users.save(&self.user_path)}
}
#[cfg(test)] mod tests{
 use super::*;
 #[test] fn cardinality(){let mut h=Hll::new();assert_eq!(h.estimate(),0);h.add("a");h.add("a");assert_eq!(h.estimate(),1);}
}
