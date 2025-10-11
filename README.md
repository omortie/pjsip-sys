### Description

FFI bindings for the [PJSIP](https://pjsip.org/) generated with [Rust Bindgen](https://rust-lang.github.io/rust-bindgen/)

#### Supported OS

- Linux
- Windows

### Note

The project configured to download and extract the required `PJSIP` binaries from GH releases based on the platform OS (Linux, Windows)
the pre-built binaries are associated version of [PJSIP](https://github.com/pjsip/pjproject/tree/680c32931fba7dae5b163c2a5da154057148d9c5) which may outdate soon.
you can configure it using [Git Submodules](https://github.com/omortie/pjsip-sys/blob/main/.gitmodules) and build them yourself.
