//! Library subsystem tests: pack parsing/resolution and all `materialize*` paths.
//!
//! Submodules mirror the production module split, plus one `support` module
//! for the shared parse/validate test helpers:
//! - [`support`] — shared target-document + validation test helpers.
//! - `add` — `sanitize_pkg` / `parse_spec` tests.
//! - `component` — `materialize` (component) tests.
//! - `token` — `materialize_token` + `collect_filter_dep_ids` tests.
//! - `action` — `materialize_action` tests.
//! - `registry` — pack parsing, embedded/project loading, and resolve tests.
//! - `themes` — embedded theme-pack tests.

mod action;
mod add;
mod component;
mod registry;
mod support;
mod themes;
mod token;
