use mlua::{Lua, Table, Value};

#[derive(thiserror::Error, Debug)]
#[error("Failed to set lua table readonly")]
pub struct ReadonlySetError(#[from] mlua::Error);

pub fn set_readonly(lua: &Lua, table: Table) -> Result<Table, ReadonlySetError> {
    let proxy = lua.create_table()?;
    let metatable = lua.create_table()?;

    metatable.set("__index", table.clone())?;
    metatable.set(
        "__newindex",
        lua.create_function(|_, (_table, _key, _value): (Table, Value, Value)| {
            Result::<(), _>::Err(mlua::Error::runtime("Attempt to modify read-only table"))
        })?,
    )?;
    metatable.set("__metatable", "Read-only table")?;

    proxy.set_metatable(Some(metatable));

    Ok(proxy)
}
