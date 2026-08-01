#[cfg(target_os = "windows")]
mod imp {
    use windows_service::service_dispatcher;
    use windows_service::define_windows_service;

    define_windows_service!(ffi_service_main, service_main);

    pub fn run() -> Result<(), windows_service::Error> {
        service_dispatcher::start("OttoUpdate", ffi_service_main)
    }

    fn service_main(_arguments: Vec<std::ffi::OsString>) {
        // Tracer-bullet service entry; full control handler wiring follows later prompts.
    }
}

#[cfg(target_os = "windows")]
pub use imp::run;

#[cfg(not(target_os = "windows"))]
pub fn run() -> Result<(), anyhow::Error> {
    Ok(())
}
