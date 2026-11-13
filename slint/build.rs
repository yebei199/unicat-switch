fn main() {
    // Exposed to dependents' build scripts as DEP_UNICAT_SWITCH_LIBRARY_PATH, so they can
    // add `@unicat` to slint-build's library paths without a build-dependency on this crate.
    // A build-dependency is compiled separately, with its own slint feature set: under
    // SLINT_LIVE_PREVIEW=1 that copy generates live-preview code against a slint built
    // without the `live-preview` feature and fails to compile.
    let ui =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("ui");
    println!(
        "cargo::metadata=library_path={}",
        ui.display()
    );

    slint_build::compile("ui/window.slint")
        .expect("ui/window.slint compiles");
}
