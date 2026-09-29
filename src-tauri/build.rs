fn main() {
    // Pass the repo's .env (e.g. INGOT_MSA_CLIENT_ID) to the compiler for option_env!
    println!("cargo:rerun-if-changed=../.env");
    for (key, value) in dotenvy::from_path_iter("../.env")
        .into_iter()
        .flatten()
        .flatten()
    {
        println!("cargo:rustc-env={key}={value}");
    }

    tauri_build::build()
}
