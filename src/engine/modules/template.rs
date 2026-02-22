use std::sync::{Arc, Mutex};

use mlua::UserData;
use tera::Tera;
use thiserror::Error;

use crate::{
    engine::modules::MountToGlobals,
    error::{ErrorReport, MutexLockError},
};

type SharedTemplatingEngine = Arc<Mutex<Tera>>;

#[derive(Debug, Clone)]
pub struct Template {
    tera: SharedTemplatingEngine,
}

#[derive(Debug, Error)]
#[error("Failed to render template")]
pub enum TemplateRenderError {
    Lock(#[from] MutexLockError),
    Rendering(#[from] tera::Error),
    TemplateArguments(#[from] TemplateArgumentsError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to prepare template arguments")]
pub enum TemplateArgumentsError {
    Lua(#[from] mlua::Error),
    InvalidArgumentName(#[from] InvalidArgumentNameError),
    InvalidArgumentType(#[from] InvalidArgumentTypeError),
}

#[derive(Debug, thiserror::Error)]
#[error("{0:?} is not a valid argument name")]
pub struct InvalidArgumentNameError(String);

#[derive(Debug, thiserror::Error)]
#[error("Value of type {0:?} is not a valid argument")]
pub struct InvalidArgumentTypeError(String);

impl Template {
    pub fn new() -> Self {
        Self {
            tera: Arc::new(Mutex::new(Tera::default())),
        }
    }

    pub fn render_string_with_lua_context(
        &self,
        template_content: &str,
        lua_context: mlua::Table,
    ) -> Result<String, TemplateRenderError> {
        let context =
            tera::Context::from_value(Self::build_template_arguments(lua_context)?.into())?;

        let mut guard = self.tera.lock().map_err(|_| MutexLockError)?;

        Ok(guard.render_str(template_content, &context)?)
    }

    fn build_template_arguments(
        table: mlua::Table,
    ) -> Result<tera::Map<String, tera::Value>, TemplateArgumentsError> {
        let mut map = tera::Map::new();

        for pair in table.pairs::<mlua::Value, mlua::Value>() {
            let (key, value) = pair?;

            let key_string = match key {
                mlua::Value::String(string) => string.to_string_lossy(),
                mlua::Value::Integer(integer) => integer.to_string(),
                mlua::Value::Number(float) => float.to_string(),
                other => Err(InvalidArgumentNameError(other.type_name().to_string()))?,
            };

            match value {
                mlua::Value::Nil => {
                    map.insert(key_string, ().into());
                }
                mlua::Value::Boolean(boolean) => {
                    map.insert(key_string, boolean.into());
                }
                mlua::Value::Integer(integer) => {
                    map.insert(key_string, integer.into());
                }
                mlua::Value::Number(number) => {
                    map.insert(key_string, number.into());
                }
                mlua::Value::String(string) => {
                    if let Ok(string) = string.to_str() {
                        map.insert(key_string, string.to_string().into());
                    }
                }
                mlua::Value::Table(table) => {
                    map.insert(key_string, Self::build_template_arguments(table)?.into());
                }
                other => Err(InvalidArgumentTypeError(other.type_name().to_string()))?,
            }
        }

        Ok(map)
    }
}

impl UserData for Template {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function(
            "render",
            |lua, (template_content, context): (mlua::Value, mlua::Table)| {
                let template = lua
                    .app_data_ref::<Self>()
                    .expect("templating engine unavailable in app data");

                let lua_to_string: mlua::Function = lua.globals().get("tostring")?;

                let template_string: mlua::String = lua_to_string.call(template_content)?;
                let template_str = template_string.to_str().map_err(|error| {
                    mlua::Error::RuntimeError(format!(
                        "template content is not valid UTF-8: {}",
                        error
                    ))
                })?;

                template
                    .render_string_with_lua_context(&template_str, context)
                    .map_err(|error| {
                        mlua::Error::RuntimeError(ErrorReport::boxed_from(error).build_report())
                    })
            },
        );
    }
}

impl MountToGlobals for Template {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        lua.set_app_data(self.clone());

        let globals = lua.globals();
        globals.set("template", self)?;

        Ok(())
    }
}
