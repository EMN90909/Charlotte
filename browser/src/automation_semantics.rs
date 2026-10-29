//! Compact accessibility-first observations for Charlotte's CDP agent.
//! Element indices are ephemeral, per observation, and never become persistent
//! browser handles. Native DOM verification is still mandatory at execution.
use serde_json::{json,Value};
use crate::browser_automation::Action;

pub const MAX_SEMANTIC_CONTROLS:usize=70;
pub const MAX_AX_NODES:usize=48;

fn cleaned(s:&str,limit:usize)->String{
    s.chars().filter(|c|!c.is_control()).take(limit).collect()
}
fn role(tag:&str,kind:&str,aria:&str)->String{
    if !aria.is_empty(){return cleaned(aria,40)}
    match tag{
        "button"=>"button",
        "a"=>"link",
        "textarea"=>"textbox",
        "select"=>"combobox",
        "input" if kind=="checkbox"=>"checkbox",
        "input" if kind=="radio"=>"radio",
        "input"=>"textbox",
        _=>"control",
    }.into()
}
/// Number visible controls in the current page observation, preserve their
/// real selectors internally and provide a short role/name index to the LLM.
pub fn index_snapshot(snapshot:&mut Value)->Result<(),String>{
    let controls=snapshot.get_mut("controls").and_then(Value::as_array_mut)
        .ok_or("No interactive controls in Chromium observation")?;
    for (i,c) in controls.iter_mut().take(MAX_SEMANTIC_CONTROLS).enumerate(){
        let tag=c.get("tag").and_then(Value::as_str).unwrap_or("").to_owned();
        let kind=c.get("type").and_then(Value::as_str).unwrap_or("").to_owned();
        let aria=c.get("role").and_then(Value::as_str).unwrap_or("").to_owned();
        let semantic=role(&tag,&kind,&aria);
        c["index"]=json!(i+1);
        c["role"]=json!(semantic);
        let name=c.get("label").and_then(Value::as_str).unwrap_or("");
        c["label"]=json!(cleaned(name,140));
    }
    controls.truncate(MAX_SEMANTIC_CONTROLS);
    Ok(())
}
/// Convert model index tools into the corresponding observed CSS path.
/// The existing unique-target, origin and live-DOM checks then apply.
pub fn resolve_index(action:Action,snapshot:&Value)->Result<Action,String>{
    let (index,typed)=match action{
        Action::ClickIndex{index}=>(index,None),
        Action::TypeIndex{index,text}=>(index,Some(text)),
        other=>return Ok(other),
    };
    if index==0||index as usize>MAX_SEMANTIC_CONTROLS{
        return Err("Accessibility element index outside observed controls".into())
    }
    let controls=snapshot.get("controls").and_then(Value::as_array)
        .ok_or("Missing current indexed controls")?;
    let match_element=controls.iter().find(|c|c["index"].as_u64()==Some(index as u64))
        .ok_or("Accessibility element index no longer available")?;
    let selector=match_element.get("selector").and_then(Value::as_str)
        .filter(|value|!value.is_empty())
        .ok_or("Element has no validated selector")?.to_owned();
    let action=if let Some(text)=typed {
        if match_element["editable"].as_bool()!=Some(true){
            return Err("Indexed element is not editable".into())
        }
        Action::Type{selector,text}
    }else{Action::Click{selector}};
    crate::browser_automation::ensure_observed_target(&action,snapshot)?;
    Ok(action)
}
/// CDP's real accessibility tree is used only for semantic context. It is not
/// a permission bypass or an untrusted node ID map.
pub fn ax_tree_command()->(&'static str,Value){
    ("Accessibility.getFullAXTree",json!({"depth":6}))
}
pub fn compact_ax(response:&Value)->Value{
    let mut filtered=Vec::new();
    if let Some(nodes)=response.get("nodes").and_then(Value::as_array){
        for node in nodes.iter().take(500){
            if filtered.len()>=MAX_AX_NODES{break}
            if node["ignored"].as_bool()==Some(true){continue}
            let role=node["role"]["value"].as_str().unwrap_or("");
            let role=role.to_ascii_lowercase();
            if !matches!(role.as_str(),"button"|"link"|"textbox"|"searchbox"|
                "checkbox"|"radio"|"combobox"|"listbox"|"menuitem"|"tab"|
                "heading"|"dialog"|"alertdialog"|"switch"|"slider"|"option"){continue}
            let name=node["name"]["value"].as_str().unwrap_or("");
            // Omit potentially sensitive editable textbox names and values;
            // the page's normal sanitized label is already available.
            if matches!(role.as_str(),"textbox"|"searchbox") {continue}
            filtered.push(json!({"role":role,"name":cleaned(name,100)}));
        }
    }
    json!(filtered)
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Risk{ReadOnly,SafeWrite,Consequential}
impl Risk{
    pub fn label(self)->&'static str{
        match self{Self::ReadOnly=>"Read only",Self::SafeWrite=>"Website interaction",
            Self::Consequential=>"Consequential action — review carefully"}
    }
}
pub fn classify(action:&Action,snapshot:&Value)->Risk{
    match action{
        Action::Observe=>Risk::ReadOnly,
        Action::Type{..}|Action::TypeIndex{..}|Action::Scroll{..}=>Risk::SafeWrite,
        Action::Navigate{..}=>Risk::SafeWrite,
        Action::Click{selector}=>{
            let details=snapshot["controls"].as_array().and_then(|xs|xs.iter().find(|v|
                v["selector"].as_str()==Some(selector.as_str())));
            let text=details.map(|v|format!("{} {}",v["label"].as_str().unwrap_or(""),
                v["href"].as_str().unwrap_or(""))).unwrap_or_default();
            if is_consequential(&text){Risk::Consequential}else{Risk::SafeWrite}
        }
        Action::ClickIndex{..}|Action::ClickAt{..}|Action::Drag{..}|Action::Dialog{..}=>
            Risk::Consequential, // Unknown coordinate/button/dialog side effects.
    }
}
fn is_consequential(text:&str)->bool{
    let lower=text.to_ascii_lowercase();
    ["buy","checkout","pay","purchase","order","delete","remove",
     "send","submit","post","publish","confirm","transfer","cancel subscription",
     "sign contract","unsubscribe"].iter().any(|word|lower.contains(word))
}
#[cfg(test)]
mod tests{
 use super::*;
 #[test]fn indices_are_bound_to_real_observed_selectors(){
     let mut snap=json!({"controls":[
         {"selector":"html > body > button","label":"Save draft","tag":"button","type":"","role":"","editable":false},
         {"selector":"html > body > input","label":"Email","tag":"input","type":"text","role":"","editable":true}
     ]});
     index_snapshot(&mut snap).unwrap();
     assert_eq!(snap["controls"][0]["index"],1);
     assert_eq!(snap["controls"][1]["role"],"textbox");
     assert!(matches!(resolve_index(Action::ClickIndex{index:1},&snap).unwrap(),Action::Click{..}));
     assert!(resolve_index(Action::TypeIndex{index:1,text:"x".into()},&snap).is_err());
     assert!(resolve_index(Action::ClickIndex{index:999},&snap).is_err());
 }
 #[test]fn action_risk_treats_unlabelled_coordinates_as_consequential(){
     let snap=json!({"controls":[{"selector":"button","label":"Delete account"}]});
     assert_eq!(classify(&Action::Click{selector:"button".into()},&snap),Risk::Consequential);
     assert_eq!(classify(&Action::ClickAt{x:4,y:5},&snap),Risk::Consequential);
 }
 #[test]fn accessibility_tree_filters_textbox_values(){
     let ax=json!({"nodes":[
         {"role":{"value":"textbox"},"name":{"value":"Secret"}},
         {"role":{"value":"button"},"name":{"value":"Submit"}},
         {"ignored":true,"role":{"value":"button"},"name":{"value":"Hidden"}}
     ]});
     assert_eq!(compact_ax(&ax),json!([{"role":"button","name":"Submit"}]));
 }
}
