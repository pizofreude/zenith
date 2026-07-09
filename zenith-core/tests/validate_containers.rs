//! Integration tests: containers validation.
//!
//! Test bodies moved verbatim from the former in-`src` `validate/check/tests/`
//! concern files; only import paths changed (`crate::`/`super::common` ->
//! `zenith_core::`/`common`).

use std::collections::BTreeMap;

mod common;

use common::*;
use zenith_core::format::format_document;

#[path = "validate_containers/frame.rs"]
mod frame;
#[path = "validate_containers/group.rs"]
mod group;
#[path = "validate_containers/semantic.rs"]
mod semantic;
