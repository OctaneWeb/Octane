//! Sizing the memory a connection uses.
//!
//! The serve loop never allocates, so every byte it needs is in two fixed
//! buffers whose sizes are part of the [`Config`] type:
//!
//! - `INPUT` holds the request being read, head and body together. It bounds
//!   the largest request that can be served, bigger ones get a `431` or `413`.
//! - `OUTPUT` holds the response head. Response bodies are not copied into it.
//!
//! Without the `std` feature there is no default: the sizes have to be
//! chosen for the target, since on bare metal they come out of a stack (or a
//! static) that is often only a few KiB. [`Config::stack_size`] says how much
//! [`Config::serve`] puts on the stack, [`Config::serve_with`] takes the
//! buffers from elsewhere instead.
//!
//! ```
//! use octane_http::config::Config;
//!
//! // 4 KiB for requests, 512 bytes for response heads
//! const CONFIG: Config<4096, 512> = Config::new();
//! assert_eq!(CONFIG.stack_size(), 4608);
//! ```
//!
//! Sizes below [`MIN_INPUT_SIZE`] or [`MIN_OUTPUT_SIZE`] don't compile:
//!
//! ```compile_fail
//! use octane_http::config::Config;
//!
//! let config = Config::<16, 512>::new();
//! ```

/// The smallest `INPUT`, enough for a short request with a `Host` header
pub const MIN_INPUT_SIZE: usize = 128;

/// The smallest `OUTPUT`, enough for any error response the server sends
/// itself, so a request that fails to parse always gets an answer
pub const MIN_OUTPUT_SIZE: usize = 128;

/// The `INPUT` size of [`DefaultConfig`]
#[cfg(feature = "std")]
pub const DEFAULT_INPUT_SIZE: usize = 16 * 1024;

/// The `OUTPUT` size of [`DefaultConfig`]
#[cfg(feature = "std")]
pub const DEFAULT_OUTPUT_SIZE: usize = 1024;

/// Sizes for hosts with large stacks, only available with `std`.
/// It takes 17 KiB of stack per connection served with [`Config::serve`].
#[cfg(feature = "std")]
pub type DefaultConfig = Config<DEFAULT_INPUT_SIZE, DEFAULT_OUTPUT_SIZE>;

/// The buffer sizes of a connection, see the [module docs](self)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config<const INPUT: usize, const OUTPUT: usize> {
    _sized: (),
}

impl<const INPUT: usize, const OUTPUT: usize> Config<INPUT, OUTPUT> {
    /// Fails to compile if a size is below its minimum
    pub const fn new() -> Self {
        assert_sizes::<INPUT, OUTPUT>();
        Self { _sized: () }
    }

    pub const fn input_size(&self) -> usize {
        INPUT
    }

    pub const fn output_size(&self) -> usize {
        OUTPUT
    }

    /// The bytes of stack [`Config::serve`] uses for its buffers, on top of
    /// what the transport and the handler need
    pub const fn stack_size(&self) -> usize {
        INPUT + OUTPUT
    }

    /// Zeroed buffers of the configured sizes
    pub const fn buffers(&self) -> Buffers<INPUT, OUTPUT> {
        Buffers::new()
    }
}

/// The buffers one connection is served with. `new` is a `const fn`, so on a
/// target with a small stack they can be put in a `static` and lent to
/// [`Config::serve_with`].
pub struct Buffers<const INPUT: usize, const OUTPUT: usize> {
    pub(crate) input: [u8; INPUT],
    pub(crate) output: [u8; OUTPUT],
}

impl<const INPUT: usize, const OUTPUT: usize> Buffers<INPUT, OUTPUT> {
    /// Fails to compile if a size is below its minimum
    pub const fn new() -> Self {
        assert_sizes::<INPUT, OUTPUT>();
        Self {
            input: [0; INPUT],
            output: [0; OUTPUT],
        }
    }
}

// The sizes are still type parameters, so `Default` doesn't make them optional
impl<const INPUT: usize, const OUTPUT: usize> Default for Config<INPUT, OUTPUT> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const INPUT: usize, const OUTPUT: usize> Default for Buffers<INPUT, OUTPUT> {
    fn default() -> Self {
        Self::new()
    }
}

const fn assert_sizes<const INPUT: usize, const OUTPUT: usize>() {
    const {
        assert!(
            INPUT >= MIN_INPUT_SIZE,
            "the INPUT buffer must be at least MIN_INPUT_SIZE bytes"
        );
        assert!(
            OUTPUT >= MIN_OUTPUT_SIZE,
            "the OUTPUT buffer must be at least MIN_OUTPUT_SIZE bytes"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::response::error_response;
    use crate::StatusCode;

    #[test]
    fn the_smallest_output_fits_every_error_response() {
        // the longest reason phrase among the statuses the server sends
        for status in [
            StatusCode::RequestHeaderFieldsTooLarge,
            StatusCode::HttpVersionNotSupported,
            StatusCode::InternalServerError,
            StatusCode::NetworkAuthenticationRequired,
        ] {
            let mut out = [0u8; MIN_OUTPUT_SIZE];
            assert!(error_response(&mut out, status).is_ok(), "{:?}", status);
        }
    }

    #[test]
    fn sizes_are_reported() {
        const CONFIG: Config<4096, 256> = Config::new();
        assert_eq!(CONFIG.input_size(), 4096);
        assert_eq!(CONFIG.output_size(), 256);
        assert_eq!(CONFIG.stack_size(), 4352);
        let buffers = CONFIG.buffers();
        assert_eq!((buffers.input.len(), buffers.output.len()), (4096, 256));
    }
}
