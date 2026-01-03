use std::{io::Write, path::PathBuf};

static TYPES_LUA: &str = include_str!("types.lua");
static ARC_LUA: &str = include_str!("arc.lua");
static LUA_RC_JSON: &str = include_str!(".luarc.json");

#[derive(Debug, thiserror::Error)]
#[error("Project initialization failed")]
pub enum InitializationFailure {
    RootDirectory(#[from] RootDirectoryCreationError),
    LspTypes(#[from] LspTypesCreationError),
    ArcLua(#[from] ArcLuaCreationError),
    LuaRc(#[from] LuaRcCreationError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to create project root directory")]
pub struct RootDirectoryCreationError(#[source] std::io::Error);

#[derive(Debug, thiserror::Error)]
#[error("Failed to write lsp types file")]
pub struct LspTypesCreationError(#[source] std::io::Error);

#[derive(Debug, thiserror::Error)]
#[error("Failed to write initial arc.lua")]
pub struct ArcLuaCreationError(#[source] std::io::Error);

#[derive(Debug, thiserror::Error)]
#[error("Failed to write initial arc.lua")]
pub struct LuaRcCreationError(#[source] std::io::Error);

pub fn init_project(project_root: PathBuf) -> Result<(), InitializationFailure> {
    std::fs::create_dir_all(&project_root).map_err(RootDirectoryCreationError)?;

    let mut lsp_types_path = project_root.clone();
    lsp_types_path.push("types.lua");

    let mut lsp_types_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(lsp_types_path)
        .map_err(LspTypesCreationError)?;
    lsp_types_file
        .write_all(
            TYPES_LUA
                .replacen("{version}", env!("CARGO_PKG_VERSION"), 1)
                .as_bytes(),
        )
        .map_err(LspTypesCreationError)?;

    let mut arc_lua_path = project_root.clone();
    arc_lua_path.push("arc.lua");

    let mut arc_lua_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(arc_lua_path)
        .map_err(LspTypesCreationError)?;
    arc_lua_file
        .write_all(ARC_LUA.as_bytes())
        .map_err(LspTypesCreationError)?;

    let mut lua_rc_path = project_root.clone();
    lua_rc_path.push(".luarc.json");

    let mut lua_rc_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(lua_rc_path)
        .map_err(LspTypesCreationError)?;
    lua_rc_file
        .write_all(LUA_RC_JSON.as_bytes())
        .map_err(LspTypesCreationError)?;

    Ok(())
}
