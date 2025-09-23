fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Rebuild if i18n files change
    println!("cargo:rerun-if-changed=i18n");

    // Copy all icons from resource folder to output directory
    let out_dir = std::env::var("OUT_DIR")?;
    let res_dir = "../res/icons/bundled/applet-button";
    let dest_dir = format!("{}/icons", out_dir);

    // Create destination directory if it doesn't exist
    std::fs::create_dir_all(&dest_dir)?;

    // Copy all files recursively
    for entry in walkdir::WalkDir::new(res_dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let rel_path = entry.path().strip_prefix(res_dir)?;
            let dest_path = std::path::Path::new(&dest_dir).join(rel_path);
            if let Some(parent) = dest_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(entry.path(), &dest_path)?;
        }
    }

    // Emit version information (if not cached by just vendor)
    let mut vergen = vergen::EmitBuilder::builder();

    println!("cargo:rerun-if-env-changed=VERGEN_GIT_COMMIT_DATE");
    if std::env::var_os("VERGEN_GIT_COMMIT_DATE").is_none() {
        vergen.git_commit_date();
    }

    println!("cargo:rerun-if-env-changed=VERGEN_GIT_SHA");
    if std::env::var_os("VERGEN_GIT_SHA").is_none() {
        vergen.git_sha(false);
    }
    vergen.fail_on_error().emit()?;
    Ok(())
}
