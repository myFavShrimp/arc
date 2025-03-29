use mlua::UserData;

pub struct Env;

impl Env {
    fn get(var_name: &str) -> Option<String> {
        std::env::var(var_name).ok()
    }
}

impl UserData for Env {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("get", |_, var_name: String| Ok(Self::get(&var_name)));
    }
}
