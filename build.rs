#[cfg(target_os = "windows")]
fn main() {
    println!("cargo:rerun-if-changed=icon.ico");

    let mut res = winres::WindowsResource::new();
    res.set_icon("icon.ico");
    res.compile().unwrap();
}

#[cfg(not(target_os = "windows"))]
fn main() {}
