/// Builds standard Aikar's high-performance server G1GC flags
pub fn build_aikar_gc_flags(memory_min_mb: u32, memory_max_mb: u32) -> Vec<String> {
    vec![
        format!("-Xms{memory_min_mb}M"),
        format!("-Xmx{memory_max_mb}M"),
        "-XX:+UseG1GC".into(),
        "-XX:+ParallelRefProcEnabled".into(),
        "-XX:MaxGCPauseMillis=200".into(),
        "-XX:+UnlockExperimentalVMOptions".into(),
        "-XX:+DisableExplicitGC".into(),
        "-XX:+AlwaysPreTouch".into(),
        "-XX:G1NewSizePercent=30".into(),
        "-XX:G1MaxNewSizePercent=40".into(),
        "-XX:G1ReservePercent=20".into(),
        "-XX:G1HeapWastePercent=5".into(),
        "-XX:G1MixedGCCountTarget=4".into(),
        "-XX:InitiatingHeapOccupancyPercent=15".into(),
        "-XX:G1MixedGCLiveThresholdPercent=90".into(),
        "-XX:G1RSetUpdatingPauseTimePercent=5".into(),
        "-XX:SurvivorRatio=32".into(),
        "-XX:+PerfDisableSharedMem".into(),
        "-XX:MaxTenuringThreshold=1".into(),
        "-Dusing.aikars.flags=https://mcflags.emc.gs".into(),
        "-Daikars.new.flags=true".into(),
    ]
}

/// Builds Java flags required for newer Java runtimes (modules / unsafe memory)
pub fn build_java_version_flags(required_java: u32) -> Vec<String> {
    let mut flags = Vec::new();
    if required_java >= 24 {
        flags.push("--sun-misc-unsafe-memory-access=allow".into());
    }
    if required_java >= 17 {
        flags.push("--enable-native-access=ALL-UNNAMED".into());
    }
    flags
}
