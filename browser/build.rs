fn main() {
    #[cfg(target_os = "windows")]
    embed_resource::compile("resources/windows/charlotte.rc", embed_resource::NONE)
        .manifest_optional()
        .unwrap();
}
