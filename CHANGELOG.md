# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.3] - 2026-04-12

- Added support for no-receiver trait method forwarding via `#[fallback]` on enum variants
- Added macro error when `configure` is renamed inside `declare!`
- Fixed generic `From`/`TryInto` code generation for configured enums
- Updated conversion generation to use `TryFrom<Enum>` impls (enabling `.try_into()` via blanket impl)
- Made generated `TryInto` error payload field public with enum-matching visibility for value recovery
- Fixed receiver-method `-> Self` forwarding to wrap returned inner values into the matching enum variant

## [0.2.2] - 2026-03-09

- Reduced MSRV to Rust 1.85.1

## [0.2.1] - 2026-03-02

- Added `inherent(visibility)` to configure to allow configuring inherent impl visibility
- Fixed rust-analyzer not recognizing attribute macros

## [0.2.0] - 2026-02-28

- Added support for remote traits with `#[disponent::remote]`

## [0.1.0] - 2026-02-28

Initial release of `disponent`
