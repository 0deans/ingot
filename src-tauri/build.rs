fn main() {
    // Pass the repo's .env (e.g. INGOT_MSA_CLIENT_ID) to the compiler for option_env!
    println!("cargo:rerun-if-changed=../.env");
    if let Ok(vars) = dotenvy::from_path_iter("../.env") {
        for (key, value) in vars.flatten() {
            println!("cargo:rustc-env={key}={value}");
        }
    }

    tauri_build::build();

    // Tauri embeds this dependency in the application resource. Unit-test
    // executables also need it when exercising IPC through the mock runtime.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
        // The main binary already has Tauri's resource manifest; embedding a
        // second manifest would create a duplicate resource, including bin tests.
        println!("cargo:rustc-link-arg-bin=ingot=/MANIFEST:NO");
    }
}
