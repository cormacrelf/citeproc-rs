macro_rules! utf8_from_raw {
    ($ptr:expr, $len:expr) => {{
        assert!(!$ptr.is_null());
        let ptr = ($ptr) as *const u8;
        let len = $len;
        let slice = std::slice::from_raw_parts(ptr, len);
        let string =
            std::str::from_utf8(slice).expect(concat!(stringify!($ptr), " must be valid UTF-8"));
        string
    }};
}

/// Wraps a function that should be available from C in no_mangle/extern, and a panic::catch_unwind
/// block so panics cannot unwind outside the Rust stack into C territory. Any caught panics will
/// abort the process.
///
/// Taken from hyperium/hyper src/ffi/macros.rs
macro_rules! ffi_fn {
    ($(#[$doc:meta])* fn $name:ident($($arg:ident: $arg_ty:ty),*) -> $ret:ty $body:block) => {
        $(#[$doc])*
        #[no_mangle]
        pub extern fn $name($($arg: $arg_ty),*) -> $ret {
            use std::panic::{self, AssertUnwindSafe};

            // The blanket AssertUnwindSafe is only possible because if we ever do panic we abort,
            // so you can never use a returned value again anyway.
            // If you remove the abort, you have to remove the AssertUnwindSafe.
            match panic::catch_unwind(AssertUnwindSafe(move || $body)) {
                Ok(v) => v,
                Err(_) => {
                    // TODO: We shouldn't abort, but rather figure out how to
                    // convert into the return type that the function errored.
                    eprintln!("panic unwind caught, aborting");
                    std::process::abort();
                }
            }
        }
    };

    ($(#[$doc:meta])* fn $name:ident($($arg:ident: $arg_ty:ty),*) $body:block) => {
        ffi_fn!($(#[$doc])* fn $name($($arg: $arg_ty),*) -> () $body);
    };
}
