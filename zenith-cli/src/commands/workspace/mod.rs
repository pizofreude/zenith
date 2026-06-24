//! Pure logic for `zenith workspace scratch`, `zenith workspace candidate`,
//! `zenith workspace promote`, `zenith workspace bundle`, and
//! `zenith workspace unbundle`.
//!
//! Submodules:
//! - [`scratch`] — `zenith workspace scratch new/list/show`
//! - [`candidate`] — `zenith workspace candidate` (set lifecycle status)
//! - [`promote`] — `zenith workspace promote` (merge a selected candidate into a page)
//! - [`bundle`] — `zenith workspace bundle/unbundle`

pub(crate) mod bundle;
mod candidate;
mod promote;
pub(crate) mod scratch;

pub use bundle::{bundle_doc, bundle_doc_in, unbundle_doc, unbundle_doc_in};
pub use candidate::{candidate_set_status, candidate_set_status_in};
pub use promote::{promote, promote_in};
pub use scratch::{
    scratch_list, scratch_list_in, scratch_new, scratch_new_in, scratch_show, scratch_show_in,
};
