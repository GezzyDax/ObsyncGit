fn main() {
    if std::env::var_os("CARGO_FEATURE_GUI").is_some() {
        slint_build::compile("ui/configurator.slint").expect("failed to compile Slint UI");
    }
}
