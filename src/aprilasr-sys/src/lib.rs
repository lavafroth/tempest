//! Low-level FFI bindings for the [april-asr](https://github.com/abb128/april-asr) C api (libaprilasr).
//!
//! Documentation: [stable](https://docs.rs/aprilasr-sys/)

/// Foreign Function Interface module
#[allow(unused, non_snake_case, non_camel_case_types, non_upper_case_globals)]
pub mod ffi {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    pub fn it_can_initialize() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(unsafe { ffi::aam_api_init(1) }, ());
        Ok(())
    }
}
