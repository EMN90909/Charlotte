//! OS keychain-backed API secrets. Never store credentials in SQLite.
const SERVICE: &str = "com.noethric.charlotte";
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn entry(id: &str) -> Result<keyring::Entry> {
    if id.is_empty() || id.len() > 256 ||
       !id.chars().all(|ch| ch.is_ascii_alphanumeric() ||
          matches!(ch, '-' | '_' | ':' | '.')) {
        return Err("Invalid credential identifier".into());
    }
    Ok(keyring::Entry::new(SERVICE, id)?)
}
pub fn store(id: &str, value: &str) -> Result<()> {
    entry(id)?.set_password(value)?;
    Ok(())
}
pub fn retrieve(id: &str) -> Result<String> { Ok(entry(id)?.get_password()?) }
pub fn remove(id: &str) -> Result<()> {
    entry(id)?.delete_credential()?;
    Ok(())
}
