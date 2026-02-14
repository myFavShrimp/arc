use std::path::PathBuf;

use mlua::UserData;

use crate::{
    engine::delegator::{error::FfiError, operator::FileSystemOperator},
    error::ErrorReport,
};

#[derive(Clone)]
pub struct Directory {
    pub path: PathBuf,
    pub file_system_operator: FileSystemOperator,
}

impl UserData for Directory {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.path.clone()));
        fields.add_field_method_set("path", |_, this, new_path: PathBuf| {
            this.file_system_operator
                .rename(&this.path, &new_path)
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });

        fields.add_field_method_get("permissions", |_, this| {
            this.file_system_operator
                .metadata(&this.path)
                .map(|maybe_metadata| maybe_metadata.map(|metadata| metadata.permissions))
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });
        fields.add_field_method_set("permissions", |_, this, mode: u32| {
            this.file_system_operator
                .set_permissions(&this.path, mode)
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });

        fields.add_field_method_get("file_name", |_, this| {
            Ok(this.file_system_operator.file_name(&this.path))
        });
        fields.add_field_method_set("file_name", |_, this, new_name: String| {
            this.file_system_operator
                .set_file_name(&this.path, &new_name)
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("create", |_, this, (): ()| {
            this.file_system_operator
                .create_directory(&this.path)
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });
        methods.add_method("remove", |_, this, (): ()| {
            this.file_system_operator
                .remove_directory(&this.path)
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });
        methods.add_method("metadata", |_, this, (): ()| {
            this.file_system_operator
                .metadata(&this.path)
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });
        methods.add_method("entries", |lua, this, (): ()| {
            let directory_entries = this
                .file_system_operator
                .list_directory(&this.path)
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })?;

            let mut result = Vec::with_capacity(directory_entries.len());

            for entry in directory_entries {
                result.push(
                    entry.into_lua(lua).map_err(|e| {
                        mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report())
                    })?,
                );
            }

            Ok(result)
        });
        methods.add_method("parent", |_, this, (): ()| {
            this.file_system_operator
                .parent_directory(&this.path)
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });
        methods.add_method("exists", |_, this, (): ()| {
            this.file_system_operator
                .metadata(&this.path)
                .map(|maybe_metadata| maybe_metadata.is_some())
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });
    }
}
