// Copyright 2015 Sean Patrick Santos
//
// Licensed under the Apache License, Version 2.0.
// License available at:
// http://www.apache.org/licenses/LICENSE-2.0
//
// Copyright and license information should be present in the top
// level directory of this package.

//! # probe-c-api
//!
//! The `probe-c-api` library creates and runs short test scripts to examine the
//! representation of types and the representations and values of constants in a
//! C library's API. The main goal is to assist Rust-based build systems,
//! especially Cargo build scripts, in producing bindings to C libraries
//! automatically. Nonetheless, this approach may be extended in the nebulous
//! future, e.g. by probing other aspects of a C API, by probing features in
//! other C-like languages (especially C++), or by adding features to aid in
//! writing bindings for other languages.

#![warn(missing_copy_implementations, missing_debug_implementations)]
#![warn(trivial_casts, trivial_numeric_casts, unused_extern_crates)]
#![warn(unused_import_braces)]
#![warn(variant_size_differences)]
#![deny(missing_docs)]

extern crate rand;

use std::boxed::Box;
use std::default::Default;
use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};

use rand::random;

pub use NewProbeError::*;

// FIXME? It's not clear whether simply aliasing the standard library types will
// provide the functionality we want from `CommandResult`, so we could hedge our
// bets by making `CommandResult` an opaque wrapper, leaving us the freedom to
// change the representation later.
//
// On the other hand, we definitely want it to be easy to construct a
// `CommandResult` from an `io::Result<process::Output>`, even if they aren't
// identical types. Also, the standard library types are actually quite good for
// this purpose, and it's not clear what more we could want!

/// Result of compilation and run commands. Currently just an alias for the
/// standard library types used by `std::process`, since in most cases we only
/// want to know:
///
///  1. Were we able to run the command at all? (If not, we'll have
///     `io::Result::Err(..)`, probably of kind `NotFound` or
///     `PermissionDenied`.)
///
///  2. If so, did the command exit with an error? (Check `status` on
///     `process::Output`.)
///
///  3. And what did the command report? (Check `stdout` and `stderr` on
///     `process::Output`.)
pub type CommandResult = io::Result<process::Output>;

/// Errors that can occur during probe construction.
#[derive(Debug)]
pub enum NewProbeError {
    /// Error returned if we cannot get the metadata for a work directory.
    WorkDirMetadataInaccessible(io::Error),
    /// Error returned if the path given for a work directory does not actually
    /// correspond to a directory.
    WorkDirNotADirectory(PathBuf),
}

impl fmt::Display for NewProbeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            WorkDirMetadataInaccessible(ref error) => {
                f.write_fmt(
                    format_args!("NewProbeError: fs::metadata returned {}",
                                 error)
                )
            }
            WorkDirNotADirectory(ref path) => {
                f.write_fmt(
                    format_args!("NewProbeError: \"{:?}\" is not a directory",
                                 path)
                )
            }
        }
    }
}

impl Error for NewProbeError {
    fn description(&self) -> &str {
        match *self {
            WorkDirMetadataInaccessible(..) => "could not query metadata from
                                                the provided work directory",
            WorkDirNotADirectory(..) => "the path in this context must be a
                                         directory",
        }
    }
    fn cause(&self) -> Option<&Error> {
        match *self {
            WorkDirMetadataInaccessible(ref error) => Some(error),
            WorkDirNotADirectory(..) => None,
        }
    }
}

/// A struct that stores information about how to compile and run test programs.
///
/// The main functionality of `probe_c_api` is implemented using the methods on
/// `Probe`. The lifetime parameter is provided in order to allow closures to be
/// used for the compiler and run commands. If `'static` types implementing `Fn`
/// are used (e.g. function pointers), the lifetime may be `'static`.
pub struct Probe<'a> {
    work_dir: PathBuf,
    compile_to: Box<Fn(&Path, &Path) -> CommandResult + 'a>,
    run: Box<Fn(&Path) -> CommandResult + 'a>,
}

impl<'a> fmt::Debug for Probe<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_fmt(format_args!("probe_c_api::Probe in \"{:?}\"",
                                 self.work_dir))
    }
}

impl<'a> Probe<'a> {
    /// Construct a `Probe` by specifying a work directory, a method to compile
    /// a C program, and a method to run a C program.
    ///
    /// The `work_dir` argument should be a path to a directory where the probe
    /// can read, write, and execute files. We could attempt to verify this, but
    /// in practice there are too many platforms and security measures out
    /// there. So it is up to the user to figure out the difference between a
    /// failure of a test and an inability to run the test due to security
    /// measures.
    ///
    /// Files in the `work_dir` are kept from colliding via random number
    /// generator, which makes it possible to execute tests in parallel, in
    /// practice.
    ///
    /// The `compile_to` argument is responsible for taking a source file
    /// `&Path` (the first argument) and producing a runnable program at another
    /// `&Path` (the second argument). This is roughly equivalent to the shell
    /// script:
    ///
    /// ```sh
    /// gcc -c $1 -o $2
    /// ```
    ///
    /// `compile_to` should yield a `CommandResult`, which allows the exit
    /// status to be checked, and provides the standard output and error for
    /// debugging purposes.
    ///
    /// The `run` argument is responsible for running the process and yielding
    /// its status and output, again as a `CommandResult`.
    ///
    /// FIXME! Suggestions for equivalent non-POSIX examples, especially
    /// anything relevant for Windows, are welcomed.
    pub fn new<C: 'a, R: 'a>(work_dir: &Path,
                             compile_to: C,
                             run: R) -> Result<Probe<'a>, NewProbeError>
        where C: Fn(&Path, &Path) -> CommandResult,
              R: Fn(&Path) -> CommandResult {
        match fs::metadata(work_dir) {
            Ok(metadata) => if !metadata.is_dir() {
                return Err(WorkDirNotADirectory(work_dir.to_path_buf()));
            },
            Err(error) => { return Err(WorkDirMetadataInaccessible(error)); }
        }
        Ok(Probe {
            work_dir: work_dir.to_path_buf(),
            compile_to: Box::new(compile_to),
            run: Box::new(run),
        })
    }
    /// Write a byte stream to a file, then attempt to compile it.
    ///
    /// This is not terribly useful, and is provided mostly for users who simply
    /// want to reuse a closure that was used to construct the `Probe`, as well
    /// as for convenience and testing of `probe-c-api` itself.
    pub fn check_compile(&self, source: &[u8]) -> CommandResult {
        let random_suffix = random::<u64>();
        let source_path = self.work_dir.join(&format!("source-{}.c",
                                                      random_suffix));
        {
            // FIXME? Should we try putting in tests for each potential `try!`
            // error? It's hard to trigger them with Rust 1.0, since the
            // standard library's filesystem permission operations haven't been
            // stabilized yet.
            let mut file = try!(fs::File::create(&source_path));
            try!(file.write_all(source));
        }
        let program_path = self.work_dir.join("program")
                               .with_extension(env::consts::EXE_EXTENSION);
        (*self.compile_to)(&source_path, &program_path)
    }
}

/// We provide a default `Probe<'static>` that runs in an OS-specific temporary
/// directory, uses gcc, and simply runs each test.
///
/// # Panics
///
/// Panics if probe creation fails.
///
/// FIXME? Can we do better than the gcc command on Windows?
impl Default for Probe<'static> {
    fn default() -> Self {
        Probe::new(
            &env::temp_dir(),
            |source_path, program_path| {
                Command::new("gcc").arg("-c").arg(source_path)
                                   .arg("-o").arg(program_path)
                                   .output()
            },
            |program_path| {
                Command::new(program_path).output()
            },
        ).unwrap()
    }
}
