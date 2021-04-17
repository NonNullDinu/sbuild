#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/leafbuild/leafbuild/master/leaf_icon.svg",
    html_logo_url = "https://raw.githubusercontent.com/leafbuild/leafbuild/master/leaf_icon.svg"
)]
#![forbid(
    unsafe_code,
    unused_allocation,
    coherence_leak_check,
    confusable_idents,
    trivial_bounds
)]
#![deny(
    missing_docs,
    missing_crate_level_docs,
    missing_copy_implementations,
    missing_debug_implementations,
    unused_imports,
    unused_import_braces,
    deprecated,
    broken_intra_doc_links,
    where_clauses_object_safety,
    order_dependent_trait_objects,
    unconditional_panic,
    unconditional_recursion,
    indirect_structural_match
)]
#![deny(
    clippy::correctness,
    clippy::style,
    clippy::complexity,
    clippy::pedantic,
    clippy::nursery
)]
#![allow(clippy::module_name_repetitions)]

//! Interprets the [`ast`][leafbuild_ast] produced by [`leafbuild_parser`], which it is also
//! responsible of getting.
//!
//! Everything you need to do is invoke `run` on a directory and watch the magic happen, at least
//! until you get the result back.
//! The interpreter module
//! Handles everything related to interpreting the source AST.

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate thiserror;

use std::io;
use std::path::PathBuf;

use tracing::{span, Level};

use crate::diagnostics::errors::LeafParseError;
use leafbuild_core::lf_buildsys::{ConfigurationError, WriteResultsError};

use crate::handle::Handle;

mod diagnostics;
pub mod env;
pub mod handle;

include!("mod_name.rs");

/// Couldn't interpret something or validate something
#[derive(Error, Debug)]
pub enum InterpretFailure {
    /// Cannot read the source file
    #[error("cannot read file {0:?}: {1}")]
    CannotReadFile(PathBuf, #[source] io::Error),

    /// Cannot validate the configuration at the end.
    #[error(transparent)]
    Validate(#[from] ConfigurationError),

    /// Cannot write the results
    #[error(transparent)]
    CannotWriteResults(#[from] WriteResultsError),
}
/// Starts the interpreter on the given path, with the given handle and modifies the handle at the end.
/// The caller is responsible for validating and writing the results, by calling [`Handle::validate`]
/// and [`Handle::write_results`] after calling this.
/// # Errors
/// See [`InterpretFailure`]
pub fn execute_on(
    handle: &mut Handle,
    root_path: &PathBuf,
    mod_path: LfModName,
) -> Result<(), InterpretFailure> {
    let span = span!(Level::TRACE, "execute_on", path = %mod_path.0.as_str());
    let _span_guard = span.enter();
    info!("Entered {}", mod_path.0.as_str());

    let build_decl_file = root_path.join("build.leaf");
    let content = std::fs::read_to_string(build_decl_file)
        .map_err(|err| InterpretFailure::CannotReadFile(root_path.join("build.leaf"), err))?;
    let (node, errors) = leafbuild_syntax::parser::parse(&content);

    if errors.is_empty() {
        let fid = handle
            .buildsys
            .register_new_file(root_path.to_string_lossy().to_string(), content);
        let mut _frame = env::FileFrame::new(fid, mod_path);
        // internal::run_build_def(&mut frame, build_definition);
    } else {
        handle.buildsys.register_file_and_report_chain(
            &root_path.to_string_lossy().to_string(),
            &content,
            |fid| {
                errors
                    .into_iter()
                    .map(move |err| LeafParseError::from((fid, err)))
            },
        );
    }

    info!("Leaving folder {:?}", root_path);

    Ok(())
}
