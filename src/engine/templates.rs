use std::sync::{Arc, Mutex};

use mlua::UserData;
use tera::Tera;
use thiserror::Error;

use crate::error::MutexLockError;

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
        let mut context = tera::Context::new();
        Self::build_template_arguments(&mut context, lua_context, None)?;

        let mut guard = self.tera.lock().map_err(|_| MutexLockError)?;

        Ok(guard.render_str(template_content, &context)?)
    }

    fn build_template_arguments(
        context: &mut tera::Context,
        table: mlua::Table,
        prefix: Option<String>,
    ) -> Result<(), TemplateArgumentsError> {
        for pair in table.pairs::<mlua::Value, mlua::Value>() {
            let (key, value) = pair?;

            let key_string = match key {
                mlua::Value::String(s) => s.to_string_lossy(),
                mlua::Value::Integer(i) => i.to_string(),
                mlua::Value::Number(n) => n.to_string(),
                other => Err(InvalidArgumentNameError(other.type_name().to_string()))?,
            };

            let full_key = if let Some(ref prefix) = prefix {
                format!("{}.{}", prefix, key_string)
            } else {
                key_string
            };

            match value {
                mlua::Value::Nil => context.insert(&full_key, &()),
                mlua::Value::Boolean(b) => context.insert(&full_key, &b),
                mlua::Value::Integer(i) => context.insert(&full_key, &i),
                mlua::Value::Number(n) => context.insert(&full_key, &n),
                mlua::Value::String(s) => {
                    if let Ok(string) = s.to_str() {
                        context.insert(&full_key, &string.to_string())
                    }
                }
                mlua::Value::Table(t) => {
                    Self::build_template_arguments(context, t, Some(full_key))?;
                }
                other => Err(InvalidArgumentNameError(other.type_name().to_string()))?,
            }
        }

        Ok(())
    }
}

impl UserData for Templates {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "render",
            |_, this, (template_content, context): (String, mlua::Table)| match this
                .render_string_with_lua_context(&template_content, context)
            {
                Ok(result) => Ok(result),
                Err(err) => Err(mlua::Error::external(err)),
            },
        );
    }
}
