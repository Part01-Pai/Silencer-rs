fn main() {
    use std::path::PathBuf;

    if !cfg!(target_os = "windows") {
        return;
    }

    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let base = PathBuf::from(&manifest);
    let candidates = [
        base.join("silencer-rs.ico"),
        base.join("..").join("silencer-rs.ico"),
        base.join("..").join("silencer-master").join("silencer.ico"),
        base.join("silencer.ico"),
    ];

    let mut found: Option<PathBuf> = None;
    for c in &candidates {
        if c.exists() {
            found = Some(c.clone());
            break;
        }
    }

    if let Some(path) = found {
        let p = path.to_string_lossy().to_string();
        match winres::WindowsResource::new().set_icon(&p).compile() {
            Ok(_) => println!("cargo:warning=Embedded icon from {}", p),
            Err(e) => println!("cargo:warning=Failed to embed icon: {}", e),
        }
    } else {
        println!("cargo:warning=No ico found for embedding (checked manifest and parent paths)");
    }
}
