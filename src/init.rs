use std::path::PathBuf;

use crate::engine::check::DEFINITIONS;

static ARC_LUA: &str = include_str!("arc.lua");
static LUAU_RC: &str = include_str!(".luaurc");

#[derive(Debug, thiserror::Error)]
#[error("Project initialization failed")]
pub enum InitializationFailure {
    RootDirectory(#[from] RootDirectoryCreationError),
    Definitions(#[from] DefinitionsCreationError),
    ArcLua(#[from] ArcLuaCreationError),
    LuauRc(#[from] LuauRcCreationError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to create project root directory")]
pub struct RootDirectoryCreationError(#[source] std::io::Error);

#[derive(Debug, thiserror::Error)]
#[error("Failed to write type definitions file")]
pub struct DefinitionsCreationError(#[source] std::io::Error);

#[derive(Debug, thiserror::Error)]
#[error("Failed to write initial arc.lua")]
pub struct ArcLuaCreationError(#[source] std::io::Error);

#[derive(Debug, thiserror::Error)]
#[error("Failed to write .luaurc")]
pub struct LuauRcCreationError(#[source] std::io::Error);

pub fn init_project(project_root: PathBuf) -> Result<(), InitializationFailure> {
    std::fs::create_dir_all(&project_root).map_err(RootDirectoryCreationError)?;

    std::fs::write(project_root.join("arc.d.luau"), DEFINITIONS)
        .map_err(DefinitionsCreationError)?;
    std::fs::write(project_root.join("arc.lua"), ARC_LUA).map_err(ArcLuaCreationError)?;
    std::fs::write(project_root.join(".luaurc"), LUAU_RC).map_err(LuauRcCreationError)?;

    Ok(())
}
