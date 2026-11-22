fn main() {
    // oboe (cpal's Android backend) is C++ and needs the shared C++ runtime;
    // without it dlopen fails on `__cxa_pure_virtual`. Gradle packages
    // libc++_shared.so next to the backend library.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        println!("cargo:rustc-link-lib=c++_shared");
    }
    tauri_build::build();
}
