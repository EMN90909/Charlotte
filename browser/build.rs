fn main() {
    #[cfg(target_os = "windows")]
    {
        if std::path::Path::new("resources/windows/charlotte.rc").exists() {
            let res = embed_resource::compile("resources/windows/charlotte.rc", embed_resource::NONE);
            if let Err(err) = res.manifest_optional() {
                println!(
                    "cargo:warning=Skipping optional Windows resource embedding (rc.exe unavailable): {err}"
                );
            }
        }
    }
}
