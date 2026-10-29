//! Deterministic, read-only browser skills. No network calls, form mutation,
//! DOM scripting selected by a model, credentials, screenshots or file writes.
//! A skill uses the same authenticated active CEF tab with ordinary CDP
//! observation and is never allowed to expand its origin or privileges.
use serde_json::Value;
use crate::browser_automation::Context;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Skill { Summary, Links, Controls, Frames }
impl Skill {
    pub fn parse(name:&str)->Result<Self,String>{
        match name {
            "summary"=>Ok(Self::Summary),
            "links"=>Ok(Self::Links),
            "controls"=>Ok(Self::Controls),
            "frames"=>Ok(Self::Frames),
            _=>Err("Unknown built-in browser skill".into()),
        }
    }
    pub fn title(self)->&'static str{
        match self{
            Self::Summary=>"Page summary",
            Self::Links=>"Visible links",
            Self::Controls=>"Interactive controls",
            Self::Frames=>"Frame inventory",
        }
    }
}
fn cleaned(input:&str, max:usize)->String {
    input.chars().filter(|c|!c.is_control()||*c=='\n')
        .take(max).collect()
}
fn safe_line(value:&str)->String{
    // The result is plain text in the user's sidebar, escaped by the renderer.
    cleaned(value,160).replace('\n'," ")
}
pub fn format_snapshot(skill:Skill,snapshot:&Value)->Result<String,String>{
    let url=snapshot["url"].as_str().ok_or("Page has no public URL")?;
    let title=safe_line(snapshot["title"].as_str().unwrap_or("Untitled"));
    let controls=snapshot["controls"].as_array().ok_or("Page has no observable controls")?;
    let header=format!("{}\nPage: {title}\nURL: {url}\n\n",skill.title());
    let result=match skill{
        Skill::Summary=>{
            let excerpt=cleaned(snapshot["excerpt"].as_str().unwrap_or(""),3200);
            if excerpt.is_empty(){
                format!("{header}No readable page text in this document.")
            }else{
                format!("{header}{excerpt}")
            }
        },
        Skill::Links=>{
            let mut links=Vec::new();
            for item in controls.iter().take(70){
                if item["tag"].as_str()!=Some("a"){continue}
                let href=item["href"].as_str().unwrap_or("");
                if href.is_empty()||links.iter().any(|(url,_):&(String,String)|url==href){continue}
                let label=safe_line(item["label"].as_str().unwrap_or("Link"));
                links.push((href.to_owned(),label));
            }
            if links.is_empty(){format!("{header}No visible links in the bounded observation.")}
            else{
                let mut out=header;
                for (i,(url,label)) in links.into_iter().take(35).enumerate(){
                    out.push_str(&format!("{}. {} — {}\n",i+1,label,url));
                }
                out
            }
        },
        Skill::Controls=>{
            let mut out=header;
            for (i,item) in controls.iter().take(50).enumerate(){
                let label=safe_line(item["label"].as_str().unwrap_or(""));
                let role=safe_line(item["role"].as_str().unwrap_or(item["tag"].as_str().unwrap_or("control")));
                let kind=if item["editable"].as_bool()==Some(true){"editable"}else{"action"};
                out.push_str(&format!("{}. {} ({}, {})\n",i+1,label,role,kind));
            }
            if controls.is_empty(){out.push_str("No visible interactive controls in the bounded observation.")}
            out
        },
        Skill::Frames=>{
            let frames=snapshot["frames"].as_array();
            let mut out=header;
            for (i,frame) in frames.into_iter().flatten().take(24).enumerate(){
                let origin=frame["url"].as_str().unwrap_or("[URL not exposed]");
                let kind=if frame["sameOrigin"].as_bool()==Some(true){"same-origin DOM accessible"}
                    else{"DOM isolated; visual only"};
                out.push_str(&format!("{}. {} — {}\n",i+1,origin,kind));
            }
            if out==header{out.push_str("No subframes reported by the bounded observation.")}
            out
        },
    };
    Ok(result.chars().take(6500).collect())
}
/// Never receives mutable browser actions; always reads the *current* active
/// authorized tab. Use only from a user-triggered, CSRF-checked sidebar route.
pub fn execute(skill:Skill,context:&Context)->Result<String,String>{
    let snapshot=context.observe_settled()?;
    format_snapshot(skill,&snapshot)
}
#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::json;
    fn example()->Value{json!({"url":"https://site.example/page","title":"Demo",
        "excerpt":"Hello reader",
        "controls":[
            {"tag":"a","label":"Home","href":"https://site.example/home","role":"link"},
            {"tag":"a","label":"Home duplicate","href":"https://site.example/home"},
            {"tag":"button","label":"Continue","role":"button","editable":false}
        ],
        "frames":[{"url":"https://other.example/frame","sameOrigin":false}]
    })}
    #[test]fn unknown_skills_are_rejected(){assert!(Skill::parse("arbitrary_js").is_err());}
    #[test]fn link_skill_deduplicates_and_never_reads_form_values(){
        let out=format_snapshot(Skill::Links,&example()).unwrap();
        assert_eq!(out.matches("https://site.example/home").count(),1);
        assert!(!out.contains("password"));
    }
    #[test]fn isolated_frames_are_not_misrepresented(){
        let out=format_snapshot(Skill::Frames,&example()).unwrap();
        assert!(out.contains("DOM isolated; visual only"));
    }
    #[test]fn summary_does_not_hallucinate_page_text(){
        let out=format_snapshot(Skill::Summary,&example()).unwrap();
        assert!(out.contains("Hello reader"));
        assert!(!out.contains("Continue"));
    }
}
