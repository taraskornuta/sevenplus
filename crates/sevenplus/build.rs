fn main() {
    // Embed the application icon into Windows executables.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icons/sevenplus.ico");
        res.set("FileDescription", "SevenPlus Calculator");
        res.compile().expect("compile Windows resources");
    }
    println!("cargo:rerun-if-changed=../../assets/icons/sevenplus.ico");
}
