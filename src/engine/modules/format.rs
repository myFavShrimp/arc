mod env;
mod json;
mod toml;
mod url;
mod yaml;

use mlua::UserData;

use crate::engine::modules::MountToGlobals;

pub struct Format;

impl UserData for Format {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("json", |lua, _| lua.create_userdata(json::Json));
        fields.add_field_method_get("toml", |lua, _| lua.create_userdata(toml::Toml));
        fields.add_field_method_get("yaml", |lua, _| lua.create_userdata(yaml::Yaml));
        fields.add_field_method_get("url", |lua, _| lua.create_userdata(url::Url));
        fields.add_field_method_get("env", |lua, _| lua.create_userdata(env::Env));
    }
}

impl MountToGlobals for Format {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();
        globals.set("format", self)?;

        Ok(())
    }
}
