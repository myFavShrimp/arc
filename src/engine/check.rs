use std::ffi::{CStr, CString, NulError, c_char};

unsafe extern "C" {
    fn arc_check(
        definitions: *const c_char,
        module_name: *const c_char,
        source: *const c_char,
    ) -> *mut c_char;
    fn arc_free(error_list: *mut c_char);
}

pub const DEFINITIONS: &str = include_str!("../arc.d.luau");

#[derive(Debug, thiserror::Error)]
#[error("Type check failed")]
pub enum CheckError {
    ModuleName(#[from] ModuleNameNulError),
    Source(#[from] SourceNulError),
    TypeErrors(#[from] TypeErrors),
}

#[derive(Debug, thiserror::Error)]
#[error("Module name contains a nul byte")]
pub struct ModuleNameNulError(#[source] NulError);

#[derive(Debug, thiserror::Error)]
#[error("Script source contains a nul byte")]
pub struct SourceNulError(#[source] NulError);

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct TypeErrors(pub String);

pub fn check(module_name: &str, source: &str) -> Result<(), CheckError> {
    let definitions = CString::new(DEFINITIONS).expect("arc.d.luau contains a nul byte");
    let module_name = CString::new(module_name).map_err(ModuleNameNulError)?;
    let source = CString::new(source).map_err(SourceNulError)?;

    let error_list_pointer =
        unsafe { arc_check(definitions.as_ptr(), module_name.as_ptr(), source.as_ptr()) };
    if error_list_pointer.is_null() {
        return Ok(());
    }

    let error_list = unsafe { CStr::from_ptr(error_list_pointer) }
        .to_string_lossy()
        .into_owned();
    unsafe { arc_free(error_list_pointer) };

    Err(TypeErrors(error_list).into())
}
