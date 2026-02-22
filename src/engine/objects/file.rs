use std::path::PathBuf;

use mlua::UserData;

use super::file_content::FileContent;
use crate::{
    engine::delegator::{error::FfiError, operator::FileSystemOperator},
    error::ErrorReport,
};

#[derive(Clone)]
pub struct File {
    pub path: PathBuf,
    pub file_system_operator: FileSystemOperator,
}

const FILE_CONTENT_ASSIGNMENT_TYPE_ERROR: &str =
    "Expected FileContent or string for content setter";

impl UserData for File {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("path", |_, this| Ok(this.path.clone()));
        fields.add_field_method_set("path", |_, this, new_path: PathBuf| {
            this.file_system_operator
                .rename(&this.path, &new_path)
                .map_err(|error| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                    )
                })
        });

        fields.add_field_method_get("file_name", |_, this| {
            Ok(this.file_system_operator.file_name(&this.path))
        });
        fields.add_field_method_set("file_name", |_, this, new_name: String| {
            this.file_system_operator
                .set_file_name(&this.path, &new_name)
                .map_err(|error| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                    )
                })
        });

        fields.add_field_method_get("content", |_, this| {
            Ok(FileContent {
                path: this.path.clone(),
                file_system_operator: this.file_system_operator.clone(),
            })
        });
        fields.add_field_method_set("content", |_, this, value: mlua::Value| match value {
            mlua::Value::UserData(user_data) => {
                let source = user_data.borrow::<FileContent>().map_err(|_| {
                    mlua::Error::RuntimeError(FILE_CONTENT_ASSIGNMENT_TYPE_ERROR.to_string())
                })?;

                source
                    .file_system_operator
                    .stream_to_other(&source.path, &this.file_system_operator, &this.path)
                    .map_err(|error| {
                        mlua::Error::RuntimeError(
                            ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                        )
                    })?;

                Ok(())
            }
            mlua::Value::String(string) => {
                this.file_system_operator
                    .write_file(&this.path, &string.as_bytes())
                    .map_err(|error| {
                        mlua::Error::RuntimeError(
                            ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                        )
                    })?;

                Ok(())
            }
            _ => Err(mlua::Error::RuntimeError(
                FILE_CONTENT_ASSIGNMENT_TYPE_ERROR.to_string(),
            )),
        });

        fields.add_field_method_get("permissions", |_, this| {
            this.file_system_operator
                .metadata(&this.path)
                .map(|maybe_metadata| maybe_metadata.map(|metadata| metadata.permissions))
                .map_err(|error| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                    )
                })
        });
        fields.add_field_method_set("permissions", |_, this, mode: u32| {
            this.file_system_operator
                .set_permissions(&this.path, mode)
                .map_err(|error| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                    )
                })
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("metadata", |_, this, (): ()| {
            this.file_system_operator
                .metadata(&this.path)
                .map_err(|error| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                    )
                })
        });
        methods.add_method("remove", |_, this, (): ()| {
            this.file_system_operator
                .remove_file(&this.path)
                .map_err(|error| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                    )
                })
        });
        methods.add_method("directory", |_, this, (): ()| {
            this.file_system_operator
                .parent_directory(&this.path)
                .map_err(|error| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                    )
                })
        });
        methods.add_method("exists", |_, this, (): ()| {
            this.file_system_operator
                .metadata(&this.path)
                .map(|maybe_metadata| maybe_metadata.is_some())
                .map_err(|error| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                    )
                })
        });
    }
}
