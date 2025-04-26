fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Rebuild if i18n files change
    println!("cargo:rerun-if-changed=i18n");
    
    Ok(())
}
