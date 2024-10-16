mod commands;

use pyo3::prelude::*;
use tauri::plugin::{Builder, TauriPlugin};

pub type Runtime = tauri::Wry;

#[pyclass(frozen)]
#[non_exhaustive]
pub struct AppHandle(pub tauri::AppHandle);

#[pymodule(submodule)]
pub mod pytauri {
    use super::*;

    #[pymodule_export]
    pub use crate::AppHandle;
    #[pymodule_export]
    use commands::py_invoke_handler;
}

fn get_last_segment(path: &str) -> &str {
    let segments: Vec<&str> = path.split("::").collect();
    segments.last().expect("failed to get the last segment")
}

macro_rules! get_last_segment {
    ($path:path) => {{
        {
            #[expect(unused_imports)]
            // just for IDE intellisense
            use $path as _;
        }
        get_last_segment(stringify!($path))
    }};
}

pub fn init() -> TauriPlugin<Runtime> {
    Builder::new(get_last_segment!(pytauri))
        .invoke_handler(tauri::generate_handler![commands::pyfunc])
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod foo {
        /// The name of the module.
        pub(crate) const NAME: &str = "foo";
    }

    #[test]
    fn test_get_last_segment() {
        assert_eq!(get_last_segment!(foo), foo::NAME);
        assert_eq!(get_last_segment!(self::foo), foo::NAME);
    }
}
