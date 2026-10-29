//! Curated MCP integration directory based on the user's supplied Charlotte MCP PDF.
//! Listings are connection *templates*, not working OAuth sessions or claims of
//! free vendor usage. Endpoints, access and package versions may change.
//! Nothing in this catalog is installed, spawned or authenticated automatically.
#[derive(Debug,Clone,Copy)]
pub struct Preset {
    pub id:&'static str,
    pub name:&'static str,
    pub category:&'static str,
    pub endpoint:&'static str,
    pub auth:&'static str,
    pub description:&'static str,
    pub keywords:&'static str,
}
pub static PRESETS:&[Preset]=&[
    Preset{id:"github",name:"GitHub",category:"Dev & code",endpoint:"https://api.githubcopilot.com/mcp/",auth:"oauth-or-token",description:"Code, pull requests, issues and actions",keywords:"github code repo"},
    Preset{id:"google-workspace",name:"Google Workspace",category:"Productivity",endpoint:"local:google-workspace",auth:"oauth-local",description:"Gmail, Drive, Docs, Sheets and Calendar",keywords:"gmail drive docs sheets calendar google"},
    Preset{id:"notion",name:"Notion",category:"Productivity",endpoint:"https://mcp.notion.com/mcp",auth:"oauth",description:"Pages and databases",keywords:"notes pages wiki"},
    Preset{id:"slack",name:"Slack",category:"Productivity",endpoint:"https://mcp.slack.com/mcp",auth:"oauth",description:"Channels and messages",keywords:"team chat channel"},
    Preset{id:"atlassian",name:"Atlassian Jira & Confluence",category:"Dev & code",endpoint:"https://mcp.atlassian.com/v2/mcp",auth:"oauth",description:"Issues and documentation",keywords:"jira confluence project"},
    Preset{id:"linear",name:"Linear",category:"Projects",endpoint:"https://mcp.linear.app/mcp",auth:"oauth",description:"Issues, cycles and roadmaps",keywords:"linear issues"},
    Preset{id:"linear-readonly",name:"Linear (read-only)",category:"Projects",endpoint:"https://mcp.linear.app/mcp/readonly",auth:"oauth",description:"Read-only issue inspection",keywords:"linear issues read"},
    Preset{id:"stripe",name:"Stripe",category:"Payments",endpoint:"https://mcp.stripe.com",auth:"oauth-or-token",description:"Invoices, customers and payments",keywords:"stripe invoice billing"},
    Preset{id:"supabase",name:"Supabase (read-only)",category:"Databases",endpoint:"https://mcp.supabase.com/mcp?read_only=true",auth:"oauth-or-token",description:"Read-only project/database operations",keywords:"supabase postgres db"},
    Preset{id:"figma",name:"Figma",category:"Design",endpoint:"https://mcp.figma.com/mcp",auth:"oauth",description:"Design frames and components",keywords:"figma design"},
    Preset{id:"playwright",name:"Playwright",category:"Automation",endpoint:"local:playwright",auth:"local",description:"Separate local automation server (not Charlotte's CEF tab)",keywords:"playwright test browser"},
    Preset{id:"hubspot",name:"HubSpot",category:"CRM",endpoint:"https://mcp.hubspot.com",auth:"oauth",description:"Contacts, deals and tickets",keywords:"crm hubspot sales"},
    Preset{id:"sentry",name:"Sentry",category:"Dev & code",endpoint:"https://mcp.sentry.dev/mcp",auth:"oauth",description:"Errors and releases",keywords:"error production sentry"},
    Preset{id:"asana",name:"Asana",category:"Projects",endpoint:"https://mcp.asana.com/v2/mcp",auth:"oauth",description:"Tasks and projects",keywords:"asana tasks project"},
    Preset{id:"context7",name:"Context7",category:"Dev & code",endpoint:"https://mcp.context7.com/mcp",auth:"optional-token",description:"Current library documentation",keywords:"documentation library rust code"},
    Preset{id:"cloudflare",name:"Cloudflare",category:"Cloud",endpoint:"https://mcp.cloudflare.com/mcp",auth:"oauth-or-token",description:"Workers, KV, DNS and R2",keywords:"cloudflare worker cdn"},
    Preset{id:"vercel",name:"Vercel",category:"Cloud",endpoint:"https://mcp.vercel.com",auth:"oauth",description:"Deployments and logs",keywords:"vercel deploy"},
    Preset{id:"gitlab",name:"GitLab",category:"Dev & code",endpoint:"https://gitlab.com/api/v4/mcp",auth:"oauth-or-token",description:"Repos, pipelines and merge requests",keywords:"gitlab ci"},
    Preset{id:"postgres",name:"PostgreSQL tools",category:"Databases",endpoint:"local:postgres",auth:"local",description:"Local database tools (requires configured DB)",keywords:"postgres sql query"},
    Preset{id:"sqlite",name:"SQLite tools",category:"Databases",endpoint:"local:sqlite",auth:"local-file",description:"Built-in native read-only SQLite MCP tools for a selected file",keywords:"sqlite local db"},
    Preset{id:"zapier",name:"Zapier",category:"Automation",endpoint:"https://mcp.zapier.com",auth:"oauth",description:"Zapier-connected apps, metered actions",keywords:"zapier automation"},
    Preset{id:"exa",name:"Exa",category:"Search",endpoint:"https://mcp.exa.ai/mcp",auth:"optional-token",description:"Semantic search",keywords:"exa search"},
    Preset{id:"firecrawl",name:"Firecrawl",category:"Search",endpoint:"https://mcp.firecrawl.dev/v2/mcp",auth:"optional-token",description:"Website extraction",keywords:"firecrawl scrape"},
    Preset{id:"brave-search",name:"Brave Search",category:"Search",endpoint:"local:brave-search",auth:"local",description:"Local Brave search package (needs provider setup)",keywords:"brave search"},
    Preset{id:"canva",name:"Canva",category:"Design",endpoint:"",auth:"oauth-manual",description:"Official OAuth connector; endpoint discovery needed",keywords:"canva graphics"},
    Preset{id:"miro",name:"Miro",category:"Design",endpoint:"",auth:"oauth-manual",description:"Official OAuth connector; endpoint discovery needed",keywords:"miro board"},
    Preset{id:"monday",name:"Monday.com",category:"Projects",endpoint:"https://mcp.monday.com/mcp",auth:"oauth-or-token",description:"Project boards and items",keywords:"monday boards"},
    Preset{id:"shopify-dev",name:"Shopify developer",category:"Commerce",endpoint:"local:shopify-dev",auth:"local",description:"Shopify API developer documentation",keywords:"shopify development"},
    Preset{id:"shopify-store",name:"Shopify Store UCP",category:"Commerce",endpoint:"",auth:"merchant-url",description:"Provide your store-specific authenticated MCP URL",keywords:"shopify store order"},
    Preset{id:"x-docs",name:"X API documentation",category:"Social",endpoint:"https://docs.x.com/mcp",auth:"none",description:"X documentation",keywords:"twitter x docs"},
    Preset{id:"x-api",name:"X API",category:"Social",endpoint:"https://api.x.com/mcp",auth:"oauth",description:"X posts and timelines",keywords:"twitter x post"},
    Preset{id:"paypal",name:"PayPal",category:"Payments",endpoint:"https://mcp.paypal.com/http",auth:"oauth",description:"Invoices, transactions and payments",keywords:"paypal"},
    Preset{id:"granola",name:"Granola",category:"Productivity",endpoint:"https://mcp.granola.ai/mcp",auth:"oauth",description:"Meeting notes",keywords:"granola meetings"},
    Preset{id:"amplitude",name:"Amplitude",category:"Analytics",endpoint:"https://mcp.amplitude.com/mcp",auth:"oauth",description:"Funnels and analytics",keywords:"analytics amplitude"},
    Preset{id:"clay",name:"Clay",category:"CRM",endpoint:"https://mcp.clay.earth/mcp",auth:"oauth",description:"Lead enrichment, possibly metered",keywords:"clay leads"},
    Preset{id:"box",name:"Box",category:"Productivity",endpoint:"https://mcp.box.com/mcp",auth:"oauth",description:"File search and sharing",keywords:"box files"},
    Preset{id:"grafana",name:"Grafana",category:"Cloud",endpoint:"local:grafana",auth:"local",description:"Local Grafana observability package (token required)",keywords:"grafana metrics"},
    Preset{id:"huggingface",name:"Hugging Face",category:"AI & research",endpoint:"https://huggingface.co/mcp",auth:"oauth-or-token",description:"Models, datasets and Spaces",keywords:"hugging face models"},
    Preset{id:"deepwiki",name:"DeepWiki",category:"AI & research",endpoint:"https://mcp.deepwiki.com/mcp",auth:"none",description:"Public repository documentation",keywords:"deepwiki repo"},
    Preset{id:"memory",name:"MCP Memory",category:"Local tools",endpoint:"local:memory",auth:"local",description:"Local knowledge graph storage",keywords:"memory graph"},
    Preset{id:"filesystem",name:"MCP Filesystem",category:"Local tools",endpoint:"local:filesystem",auth:"local",description:"Requires explicitly configured local filesystem root",keywords:"filesystem files"},
    Preset{id:"sequential-thinking",name:"MCP Sequential Thinking",category:"Local tools",endpoint:"local:sequential-thinking",auth:"local",description:"Local reasoning workflow",keywords:"thinking plan"},
    Preset{id:"git",name:"MCP Git",category:"Local tools",endpoint:"local:git",auth:"local",description:"Local Git repository operations",keywords:"git commits"},
];
pub fn get(id:&str)->Option<&'static Preset>{PRESETS.iter().find(|p|p.id==id)}
pub fn local_command(id:&str)->Option<(&'static str,&'static [&'static str])>{
    match id {
        "google-workspace"=>Some(("uvx",&["google-workspace-mcp"])) ,
        "playwright"=>Some(("npx",&["@playwright/mcp@latest"])) ,
        "postgres"=>Some(("npx",&["@modelcontextprotocol/server-postgres"])) ,
        "brave-search"=>Some(("npx",&["brave-search-mcp-server"])) ,
        "shopify-dev"=>Some(("npx",&["@shopify/dev-mcp@latest"])) ,
        "grafana"=>Some(("uvx",&["mcp-grafana"])) ,
        "memory"=>Some(("npx",&["@modelcontextprotocol/server-memory"])) ,
        "sequential-thinking"=>Some(("npx",&["@modelcontextprotocol/server-sequential-thinking"])) ,
        "git"=>Some(("uvx",&["mcp-server-git"])) ,
        "filesystem"=>Some(("npx",&["@modelcontextprotocol/server-filesystem"])) ,
        _=>None,
    }
}
pub fn supported(preset:&Preset)->bool{
    !preset.endpoint.is_empty()&&
    (!preset.endpoint.starts_with("local:")||local_command(preset.id).is_some())
}
/// Require explicit user action before registering any template. This is
/// NOT an OAuth sign-in or a guarantee that vendor connectivity works.
pub fn selected_url(id:&str)->Result<&'static str,String>{
    let preset=get(id).ok_or("Unknown MCP catalog entry")?;
    if !supported(preset){
        return Err("This catalog entry needs a vendor-provided endpoint or manual setup".into())
    }
    Ok(preset.endpoint)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn catalog_is_unique_and_does_not_install_anything(){
        let mut ids=std::collections::HashSet::new();
        for item in PRESETS {
            assert!(ids.insert(item.id), "Duplicate catalog id");
            if item.endpoint.starts_with("local:"){
                assert!(local_command(item.id).is_some());
            }
        }
        assert!(PRESETS.len()>=33);
    }
    #[test]fn vendor_specific_endpoints_are_not_invented(){
        assert!(selected_url("canva").is_err());
        assert!(selected_url("shopify-store").is_err());
        assert!(selected_url("github").is_ok());
    }
}
