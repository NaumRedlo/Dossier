fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon/dossier.ico");
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
