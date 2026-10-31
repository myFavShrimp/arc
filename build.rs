use std::path::PathBuf;

const LUAU_SOURCE_DIRECTORY: &str = "vendor/luau";
const CHECK_SOURCE: &str = "foam/check.cpp";
const HEADER_DIRECTORIES: [&str; 7] = [
    "Common", "Ast", "Config", "Compiler", "VM", "Bytecode", "Analysis",
];

fn main() {
    let luau_source = PathBuf::from(LUAU_SOURCE_DIRECTORY);

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++17")
        .warnings(false)
        .define("LUA_API", "extern \"C\"");

    for header_directory in HEADER_DIRECTORIES {
        build.include(luau_source.join(header_directory).join("include"));
    }

    let analysis_sources = std::fs::read_dir(luau_source.join("Analysis/src"))
        .expect("vendor/luau is missing, run scripts/fetch-luau-src.sh");
    for directory_entry in analysis_sources {
        let source_path = directory_entry
            .expect("unreadable Analysis/src entry")
            .path();
        if source_path
            .extension()
            .is_some_and(|extension| extension == "cpp")
        {
            build.file(source_path);
        }
    }

    build.file(CHECK_SOURCE);
    build.compile("luauanalysis");

    println!("cargo:rerun-if-changed={CHECK_SOURCE}");
    println!("cargo:rerun-if-changed={LUAU_SOURCE_DIRECTORY}");
}
