# Charlotte Native (Rust + CEF)

This is an incomplete Rust and CEF prototype. It does not yet replace the existing browser: the custom interface, ad blocker, AI, chat and profile migration have not been ported. Do not use it as a production browser.

| Platform | Status | Output |
| --- | --- | --- |
| Windows | planned | Setup and portable bundle |
| macOS | planned | `.app` bundle |
| Linux | planned | portable bundle |

Scripts target the host operating system; their output has not been verified. `cef-dll-sys` downloads the matching CEF runtime on the first build unless `CEF_PATH` is set. Local SQLite tables are initialized at startup, but are not yet connected to browser features.
