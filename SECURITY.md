# Security Policy

## Supported versions

REScale is a solo project in early development (pre-1.0). Only the latest
release on `main` receives fixes; older tags are not back-patched.

## Reporting a vulnerability

Please use GitHub's private vulnerability reporting: open the **Security** tab of
this repository and choose **Report a vulnerability**. Do not open a public issue
for a suspected vulnerability.

A useful report contains:

- the affected crate (`rescale-core` or `rescale-cli`) and the commit or tag,
- the exact command or Rust snippet that triggers it,
- the input file if it is small (never attach untrusted material you cannot share),
- what happens today (panic, memory growth, bad output) versus what you expected.

Expect a best-effort response within a couple of weeks. There is no bug bounty.

## Scope

The primary attack surface is **parsing untrusted audio files**. REScale performs
no network I/O and no device I/O:

- `rescale-core` is `unsafe`-free (enforced in CI) and only reads the file paths
  it is given.
- The single `unsafe` block in the workspace is a Windows-only `kernel32` FFI call
  in the CLI used to switch the console to UTF-8 and enable ANSI escapes.
- MP3 import/export shells out to an external `ffmpeg` binary; vulnerabilities in
  `ffmpeg` itself must be reported to the `ffmpeg` project.

Known correctness limits (limiter clamp behaviour, latency handling, absence of
resampling/LUFS) are documented in the README under *Known limitations*. They are
quality issues, not security issues; please report them as regular issues.
