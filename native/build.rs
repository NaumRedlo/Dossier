fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon/dossier.ico");
    println!("cargo:rerun-if-env-changed=DOSSIER_WITNESS_EXE");
    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR")).join("witness.exe");
    let built = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR")).join("../target/x86_64-pc-windows-gnu/release/witness.exe");
    println!("cargo:rerun-if-changed={}", built.display());
    let given = std::env::var_os("DOSSIER_WITNESS_EXE").map(std::path::PathBuf::from).filter(|path| path.is_file()).or_else(|| Some(built).filter(|path| path.is_file()));
    match given {
        Some(path) => {
            println!("cargo:rerun-if-changed={}", path.display());
            std::fs::copy(&path, &out).expect("the Witness program is copied in");
        }
        None => std::fs::write(&out, []).expect("an empty Witness is written"),
    }
    #[cfg(windows)]
    {
        let mut resource = winresource::WindowsResource::new();
        resource.set_icon("assets/icon/dossier.ico");
        resource.set("ProductName", "Dossier");
        resource.set("FileDescription", "Dossier");
        if let Err(why) = resource.compile() {
            panic!("the Windows resources did not compile: {why}");
        }
    }
}
