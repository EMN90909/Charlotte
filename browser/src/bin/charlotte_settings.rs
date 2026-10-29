//! Local, user-directed settings administration for native Charlotte.
//! Changes persist to the same profile as the CEF browser; no browser
//! process restart is required for future resource permission requests.
use charlotte_native::{permissions::{self, Capability, Decision}, profile, storage::Storage};
use std::{env, process};

const HELP: &str = "\
Charlotte native settings
Usage:
  charlotte-settings permission allow|deny|reset|show camera|microphone|geolocation <origin>
  charlotte-settings shields on|off|show
  charlotte-settings bookmark add <https-url> <title>
  charlotte-settings bookmark remove <https-url>
  charlotte-settings bookmark list
  charlotte-settings history list [count]
  charlotte-settings history clear
  charlotte-settings downloads list [count]
  charlotte-settings downloads clear
  charlotte-settings credential set|remove|show openai|claude|gemini
  charlotte-settings account status|signout
  charlotte-settings account signin|signup <email>
  charlotte-settings mcp tools <https-url>
  charlotte-settings mcp call <https-url> <namespaced-tool> <JSON-arguments>
  charlotte-settings profile
The browser must be restarted for changes to ad blocking; media permissions
are checked on every CEF media request. Only HTTPS origins or loopback HTTP
origins are permitted for remembered permissions.
";
fn run(args: &[String]) -> Result<(), String> {
    if args.is_empty() || args.first().map(String::as_str) == Some("--help") {
        println!("{HELP}");
        return Ok(());
    }
    profile::ensure_profile_dirs()?;
    let db = Storage::open(&profile::profile_root().join("charlotte.sqlite")).map_err(|e|e.to_string())?;
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["profile"] => println!("{}", profile::profile_root().display()),
        ["mcp","tools",endpoint] => {
            let mut client=charlotte_native::mcp::Registry::new();
            for tool in client.connect("manual",endpoint,None)?{
                println!("{}\t{}",tool.name,tool.description);
            }
        }
        ["mcp","call",endpoint,tool,arguments] => {
            let params:serde_json::Value=serde_json::from_str(arguments).map_err(|e|e.to_string())?;
            let mut client=charlotte_native::mcp::Registry::new();
            client.connect("manual",endpoint,None)?;
            let result=client.call(tool,params)?;
            println!("{}",serde_json::to_string_pretty(&result).map_err(|e|e.to_string())?);
        }
        ["account","status"] => {
            let auth=charlotte_native::auth::SupabaseAuth::configured()?;
            println!("{:#?}",auth.status());
        }
        ["account","signin"|"signup",email] => {
            let auth=charlotte_native::auth::SupabaseAuth::configured()?;
            eprintln!("Supabase account password (hidden input):");
            let password=rpassword::read_password().map_err(|e|e.to_string())?;
            let status=if args[1]=="signin"{auth.sign_in(email,&password)?}
                else{auth.sign_up(email,&password)?};
            println!("{status:#?}");
        }
        ["account","signout"] => {
            let auth=charlotte_native::auth::SupabaseAuth::configured()?;
            println!("{:#?}",auth.sign_out()?);
        }
        ["credential", action, provider] => {
            if !["openai", "claude", "gemini"].contains(provider) {
                return Err("Supported provider: openai, claude, gemini".into());
            }
            let key = format!("ai:{provider}");
            match *action {
                "set" => {
                    eprintln!("Enter {provider} API key (input is hidden):");
                    let password = rpassword::read_password().map_err(|e|e.to_string())?;
                    if password.trim().is_empty() {return Err("Empty keys are not accepted".into());}
                    charlotte_native::credentials::store(&key,&password).map_err(|e|e.to_string())?;
                    println!("{provider} API key saved in the operating system credential store");
                }
                "remove" => {
                    charlotte_native::credentials::remove(&key).map_err(|e|e.to_string())?;
                    println!("{provider} API key removed");
                }
                "show" => println!("{provider}: {}",if charlotte_native::credentials::retrieve(&key).is_ok() {"configured"}else{"not configured"}),
                _ => return Err("Credential action must be set, remove or show".into()),
            }
        },
        ["shields", "on" | "off"] => {
            db.set_setting("shields", args[1].as_str()).map_err(|e|e.to_string())?;
            println!("Shields {} (restart Charlotte to apply)", args[1]);
        }
        ["shields", "show"] => println!("{}", db.setting("shields").map_err(|e|e.to_string())?.unwrap_or_else(||"on".into())),
        ["permission", action, capability, origin] => {
            let cap = Capability::from_name(capability).ok_or("Expected camera, microphone or geolocation")?;
            match *action {
                "allow" | "deny" => permissions::set(&db, origin, cap, Decision::parse(action).unwrap())?,
                "reset" => permissions::reset(&db, origin, cap)?,
                "show" => {}
                _ => return Err("Permission action must be allow, deny, reset or show".into()),
            }
            println!("{}: {:?}", origin, permissions::get(&db, origin, cap).map_err(|e|e.to_string())?);
        }
        ["bookmark", "add", url, title] => {
            if !charlotte_native::session::valid_page_url(url) { return Err("Invalid HTTP/HTTPS URL".into()); }
            db.save_bookmark(url, title).map_err(|e|e.to_string())?;
            println!("Bookmark added: {url}");
        }
        ["bookmark", "remove", url] => {
            db.remove_bookmark(url).map_err(|e|e.to_string())?;
            println!("Bookmark removed: {url}");
        }
        ["bookmark", "list"] => for bookmark in db.bookmarks().map_err(|e|e.to_string())? {
            println!("{}\t{}", bookmark.title, bookmark.url);
        },
        ["history", "list"] | ["history", "list", _] => {
            let count = if args.len()>2 {args[2].parse::<u32>().map_err(|_|"count must be an integer")?}else{20};
            for item in db.recent_history(count.min(1000)).map_err(|e|e.to_string())? {
                println!("{}\t{}\t{}", item.visited_at, item.title, item.url);
            }
        }
        ["history", "clear"] => {
            let removed = db.clear_history().map_err(|e|e.to_string())?;
            println!("{removed} history records cleared");
        }
        ["downloads", "list"] | ["downloads", "list", _] => {
            let count = if args.len() > 2 { args[2].parse::<usize>().map_err(|_|"count must be an integer")? } else { 20 };
            for item in charlotte_native::downloads::recent(&db, count).map_err(|e|e.to_string())? {
                println!("{}	{:?}	{}	{}	{}", item.finished_at, item.status, item.received_bytes, item.source_url, item.save_path);
            }
        }
        ["downloads", "clear"] => {
            let removed = db.clear_downloads().map_err(|e|e.to_string())?;
            println!("{removed} download history records cleared (files are left untouched)");
        }
        _ => return Err(HELP.to_string()),
    }
    Ok(())
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("{error}");
        process::exit(2);
    }
}
