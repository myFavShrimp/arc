use std::sync::{Arc, Mutex};

use log::debug;
use mlua::UserData;
use tera::Tera;
use thiserror::Error;

use crate::error::{ErrorReport, MutexLockError};

#[derive(Debug, Clone)]
pub struct Templates {
    tera: Arc<Mutex<Tera>>,
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

impl Templates {
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
        debug!("Rendering template");

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
                mlua::Value::String(s) => s.to_string_lossy(),
                mlua::Value::Integer(i) => i.to_string(),
                mlua::Value::Number(n) => n.to_string(),
                other => Err(InvalidArgumentNameError(other.type_name().to_string()))?,
            };

            match value {
                mlua::Value::Nil => {
                    map.insert(key_string, ().into());
                }
                mlua::Value::Boolean(b) => {
                    map.insert(key_string, b.into());
                }
                mlua::Value::Integer(i) => {
                    map.insert(key_string, i.into());
                }
                mlua::Value::Number(n) => {
                    map.insert(key_string, n.into());
                }
                mlua::Value::String(s) => {
                    if let Ok(string) = s.to_str() {
                        map.insert(key_string, string.to_string().into());
                    }
                }
                mlua::Value::Table(t) => {
                    map.insert(key_string, Self::build_template_arguments(t)?.into());
                }
                other => Err(InvalidArgumentNameError(other.type_name().to_string()))?,
            }
        }

        Ok(map)
    }
}

impl UserData for Templates {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "render",
            |_, this, (template_content, context): (String, mlua::Table)| {
                this
                    .render_string_with_lua_context(&template_content, context)
                    .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
            },
        );
    }
}
