  What I added (the parser itself is untouched):

  - octane_http/fuzz/ — a standalone cargo-fuzz crate (its own [workspace] so it stays out of the main workspace) with one target.
  - fuzz_targets/http1x_parse.rs — feeds arbitrary bytes into RawRequest1x::parse and, whenever parsing succeeds, asserts the guarantees the rest of the server relies on: method/url/version
  are non-empty, every returned slice actually points inside the input buffer (the zero-copy contract), and the parsed pieces never exceed the input length. Any panic, out-of-bounds slice,
  or broken invariant would be reported as a crash.
  - corpus/http1x_parse/ — four valid seed requests (minimal, with headers, with body, with CRLFs inside the body) so the fuzzer starts from realistic inputs. LibFuzzer grew that to 103
  corpus entries covering 74 code edges during the run.

  To run it again (needs the nightly toolchain, which you already have):

  cd octane_http
  cargo +nightly fuzz run http1x_parse              # fuzz indefinitely
  cargo +nightly fuzz run http1x_parse -- -runs=200000 -max_len=4096   # bounded run

  I also installed cargo-fuzz (it wasn't on your machine) and added octane_http/fuzz/.gitignore for the fuzz build/artifact directories. One thing worth knowing: the run confirms the parser
  is structurally robust against arbitrary bytes — it never panics or slices out of bounds, it just returns BadRequest for garbage.

