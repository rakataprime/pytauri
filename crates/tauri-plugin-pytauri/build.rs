const COMMANDS: &[&str] = &["pyfunc"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
