fn main() {
    println!("cargo:rerun-if-changed=resources/icons/yuhina.ico");

    #[cfg(windows)]
    {
        let mut resource = winres::WindowsResource::new();
        resource.set_icon("resources/icons/yuhina.ico");
        resource.set("ProductName", "Yuhina");
        resource.set("FileDescription", "Yuhina");
        if let Err(error) = resource.compile() {
            // A missing resource compiler must not block a developer build.
            println!("cargo:warning=Yuhina icon resource skipped: {error}");
        }
    }
}
