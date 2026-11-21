//! Built-in read-only SQLite MCP-compatible adapter.
//! Unlike third-party local servers, this runs entirely in native Rust and
//! ONLY opens a user-selected SQLite file with SQLITE_OPEN_READ_ONLY.
//! Every tool call still passes the existing MCP approval gate.
use crate::mcp::Tool;
use rusqlite::{Connection,OpenFlags,types::ValueRef};
use serde_json::{json,Value};
pub struct SqliteConnection{
    db:Connection,
    pub tools:Vec<Tool>,
    pub alias:String,
    pub server_id:String,
}
impl SqliteConnection{
    pub fn open()->Result<Self,String>{
        let path=crate::credentials::retrieve("mcp-file:sqlite")
            .map_err(|_|"First select a SQLite file using charlotte-settings mcp sqlite file set <file>")?;
        let path=std::fs::canonicalize(path).map_err(|e|e.to_string())?;
        if !path.is_file(){return Err("Approved SQLite path is not a file".into())}
        let db=Connection::open_with_flags(&path,OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|e|e.to_string())?;
        db.busy_timeout(std::time::Duration::from_secs(3)).map_err(|e|e.to_string())?;
        let id="sqlite";
        let alias=crate::mcp::alias_for(id);
        let tools=vec![
            Tool{
                server_id:id.into(),original_name:"schema".into(),
                name:format!("mcp__{alias}__schema"),
                description:"List SQLite tables/views in the explicitly authorized read-only database".into(),
                input_schema:json!({"type":"object","properties":{},"additionalProperties":false}),
            },
            Tool{
                server_id:id.into(),original_name:"select".into(),
                name:format!("mcp__{alias}__select"),
                description:"Execute a single read-only SELECT or WITH query (up to 100 rows) against the user-approved SQLite file".into(),
                input_schema:json!({"type":"object","properties":{"sql":{"type":"string"}},
                    "required":["sql"],"additionalProperties":false}),
            },
        ];
        Ok(Self{db,tools,alias,server_id:id.into()})
    }
    pub fn invoke(&mut self,name:&str,args:Value)->Result<Value,String>{
        match name{
            "schema"=>{
                let mut statement=self.db.prepare(
                    "SELECT name, type FROM sqlite_master WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name LIMIT 80"
                ).map_err(|e|e.to_string())?;
                let rows=statement.query_map([],|row|Ok(json!({
                    "name":row.get::<_,String>(0)?,
                    "type":row.get::<_,String>(1)?
                }))).map_err(|e|e.to_string())?;
                let list:Result<Vec<_>,_>=rows.collect();
                Ok(json!({"objects":list.map_err(|e|e.to_string())?}))
            },
            "select"=>{
                let sql=args["sql"].as_str().ok_or("SQL query missing")?.trim();
                if sql.len()>6000{return Err("SQLite query exceeds 6000 bytes".into())}
                let lowered=sql.to_ascii_lowercase();
                if !(lowered.starts_with("select ")||lowered.starts_with("with ")||
                     lowered.starts_with("select\n")||lowered.starts_with("with\n")){
                    return Err("Only SELECT or WITH read queries are supported".into())
                }
                let mut statement=self.db.prepare(sql).map_err(|e|e.to_string())?;
                if !statement.readonly(){return Err("Query is not read-only".into())}
                if statement.column_count()>24{return Err("More than 24 columns not supported".into())}
                let names:Vec<String>=statement.column_names().iter().map(|s|s.to_string()).collect();
                let mut rows=statement.query([]).map_err(|e|e.to_string())?;
                let mut values=Vec::new();
                while values.len()<100{
                    let Some(row)=rows.next().map_err(|e|e.to_string())? else{break};
                    let mut record=serde_json::Map::new();
                    for (i,column) in names.iter().enumerate(){
                        let value=match row.get_ref(i).map_err(|e|e.to_string())?{
                            ValueRef::Null=>Value::Null,
                            ValueRef::Integer(n)=>json!(n),
                            ValueRef::Real(n)=>json!(n),
                            ValueRef::Text(bytes)=>json!(String::from_utf8_lossy(bytes)
                                .chars().take(500).collect::<String>()),
                            ValueRef::Blob(bytes)=>json!({"binary_bytes":bytes.len()}),
                        };
                        record.insert(column.to_owned(),value);
                    }
                    values.push(Value::Object(record));
                }
                let truncated=values.len()==100;
                let output=json!({"columns":names,"rows":values,"truncated":truncated});
                if output.to_string().len()>80_000{return Err("Read-only SQLite result exceeds 80 KB".into())}
                Ok(output)
            },
            _=>Err("Unknown built-in SQLite MCP tool".into())
        }
    }
}
#[cfg(test)]
mod tests{
    #[test]fn only_readonly_native_sqlite_open_flags_are_used(){
        let src=include_str!("mcp_sqlite.rs");
        let body = &src[..src.find("#[cfg(test)]").unwrap_or(src.len())];
        assert!(body.contains("SQLITE_OPEN_READ_ONLY"));
        assert!(!body.contains("SQLITE_OPEN_READ_WRITE"));
    }
}
