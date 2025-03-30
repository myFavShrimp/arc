use std::path::PathBuf;

use mlua::UserData;

use crate::{engine::delegator::operator::FileSystemOperator, error::ErrorReport};

#[derive(Clone)]
pub struct File {
    pub path: PathBuf,
    pub file_system_operator: FileSystemOperator,
}

impl UserData for File {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.path.clone()));
        fields.add_field_method_set("path", |_, this, new_path: PathBuf| {
            this.file_system_operator
                .rename(&this.path, &new_path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
        fields.add_field_method_get("content", |_, this| {
            this.file_system_operator
                .read_file(&this.path)
                .map(mlua::BString::new)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
        fields.add_field_method_set("content", |_, this, content: mlua::BString| {
            this.file_system_operator
                .write_file(&this.path, &content)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

            Ok(())
        });
        fields.add_field_method_get("permissions", |_, this| {
            this.file_system_operator
                .metadata(&this.path)
                .map(|maybe_metadata| maybe_metadata.map(|metadata| metadata.permissions))
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
        fields.add_field_method_set("permissions", |_, this, mode: u32| {
            this.file_system_operator
                .set_permissions(&this.path, mode)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("metadata", |_, this, (): ()| {
            this.file_system_operator
                .metadata(&this.path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
        methods.add_method("remove", |_, this, (): ()| {
            this.file_system_operator
                .remove_file(&this.path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
    }
}
