fn main() {
    // Pass the repo's .env (e.g. INGOT_MSA_CLIENT_ID) to the compiler for option_env!
    println!("cargo:rerun-if-changed=../.env");
    if let Ok(vars) = dotenvy::from_path_iter("../.env") {
        for (key, value) in vars.flatten() {
            println!("cargo:rustc-env={key}={value}");
        }
    }

    tauri_build::build()
}
