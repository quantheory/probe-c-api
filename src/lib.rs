// Copyright 2015 Sean Patrick Santos
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # About `probe-c-api`
//!
//! The `probe-c-api` library creates and runs short test scripts to examine the
//! representation of types and the representations and values of constants in a
//! C library's API. The main goal is to assist Rust-based build systems,
//! especially Cargo build scripts, in producing bindings to C libraries
//! automatically. Nonetheless, this approach may be extended in the nebulous
//! future, e.g. by probing other aspects of a C API, by probing features in
//! other C-like languages (especially C++), or by adding features to aid in
//! writing bindings for other languages.
//!
//! # Source file encoding
//!
//! Currently, all strings corresponding to C source code are represented as
//! UTF-8. If this is a problem, the user may modify the compile and run
//! functions commands to do appropriate translation.

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
use std::fmt::Write as FormatWrite;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::str::FromStr;

use rand::random;

use NewProbeError::*;
use CProbeError::*;

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
            WorkDirMetadataInaccessible(..) => "could not query metadata from \
                                                the provided work directory",
            WorkDirNotADirectory(..) => "the path in this context must be a \
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

// Utility to print `process::Output` in a human-readable form.
fn output_as_string(output: &process::Output) -> String {
    format!("{{ status: {:?}, stdout: {}, stderr: {} }}",
            output.status, String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr))
}

/// Outputs of both compilation and running.
pub struct CompileRunOutput {
    /// Output of the compilation phase.
    pub compile_output: process::Output,
    /// Output of the run phase. It is optional because if the compilation
    /// failed, we won't try to run at all.
    pub run_output: Option<process::Output>,
}

impl fmt::Debug for CompileRunOutput {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        f.write_fmt(
            format_args!("probe_c_api::CompileRunOutput{{ \
                          compile output: {} \
                          run output: {} \
                          }}",
                         output_as_string(&self.compile_output),
                         self.run_output.as_ref().map_or(
                             "None".to_string(),
                             |output| output_as_string(output)))
        )
    }
}

impl CompileRunOutput {
    /// Returns a probe program's standard output as a UTF-8 string.
    ///
    /// This function does not panic. If the compilation or run failed, this is
    /// reported in the error. If the program's output is not valid UTF-8, lossy
    /// conversion is performed.
    pub fn successful_run_output(&self) -> CProbeResult<String> {
        match self.run_output {
            Some(ref run_output) => {
                if run_output.status.success() {
                    Ok(String::from_utf8_lossy(&run_output.stdout).into_owned())
                } else {
                    Err(RunError(self.compile_output.clone(),
                                 run_output.clone()))
                }
            }
            None => {
                Err(CompileError(self.compile_output.clone()))
            }
        }
    }
}

// FIXME! In general there could be a lot more testing of error paths. The
// simplest way to do this would be to create a `Probe` that spoofs `Output`s
// that trigger each of these errors.

/// Error type used when a C API probing program fails to compile or run.
pub enum CProbeError {
    /// An I/O error prevented the operation from continuing.
    IoError(io::Error),
    /// Compilation failed.
    CompileError(process::Output),
    /// The probing program failed when run. The compilation output is included
    /// to assist debugging.
    RunError(process::Output, process::Output),
    /// All other errors, e.g. corrupt output from a probe program.
    OtherError(String),
}

impl fmt::Debug for CProbeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            IoError(ref error) => {
                f.write_fmt(
                    format_args!("IoError{{ {:?} }}", error)
                )
            }
            CompileError(ref output) => {
                f.write_fmt(
                    format_args!("CompileError{}", output_as_string(output))
                )
            }
            RunError(ref compile_output, ref run_output) => {
                f.write_fmt(
                    format_args!("RunError{{\
                                  compile_output: {}\
                                  run_output: {}\
                                  }}",
                                 output_as_string(compile_output),
                                 output_as_string(run_output))
                )
            }
            OtherError(ref string) => {
                f.write_fmt(
                    format_args!("OtherError{{ {} }}",
                                 string)
                )
            }
        }
    }
}

impl fmt::Display for CProbeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            IoError(ref error) => {
                f.write_fmt(
                    format_args!("I/O error: {}", error)
                )
            }
            CompileError(ref output) => {
                f.write_fmt(
                    format_args!("compilation error with output: {}",
                                 output_as_string(output))
                )
            }
            RunError(_, ref run_output) => {
                f.write_fmt(
                    format_args!("test program error with output: {}",
                                 output_as_string(run_output))
                )
            }
            OtherError(ref string) => {
                f.write_str(string)
            }
        }
    }
}

impl Error for CProbeError {
    fn description(&self) -> &str {
        match *self {
            IoError(..) => "I/O error",
            CompileError(..) => "error when compiling C probe program",
            RunError(..) => "error when running C probe program",
            OtherError(ref string) => string,
        }
    }
    fn cause(&self) -> Option<&Error> {
        match *self {
            IoError(ref error) => Some(error),
            CompileError(..) | RunError(..) | OtherError(..) => None,
        }
    }
}

impl From<io::Error> for CProbeError {
    fn from(error: io::Error) -> Self {
        IoError(error)
    }
}

/// Result type from most functions that create C probing programs.
pub type CProbeResult<T> = Result<T, CProbeError>;

/// A struct that stores information about how to compile and run test programs.
///
/// The main functionality of `probe_c_api` is implemented using the methods on
/// `Probe`. The lifetime parameter is provided in order to allow closures to be
/// used for the compiler and run commands. If `'static` types implementing `Fn`
/// are used (e.g. function pointers), the lifetime may be `'static`.
pub struct Probe<'a> {
    headers: Vec<String>,
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
    /// The `headers` argument is a vector of headers to include in every C
    /// program written by this probe. Each header should have the `<>` or `""`
    /// delimiters surrounding it.
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
    pub fn new<C: 'a, R: 'a>(headers: Vec<String>,
                             work_dir: &Path,
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
            headers: headers,
            work_dir: work_dir.to_path_buf(),
            compile_to: Box::new(compile_to),
            run: Box::new(run),
        })
    }

    // Create random paths for compilation input/output. This is intended
    // primarily to prevent two concurrently running probes from using each
    // others' files.
    fn random_source_and_exe_paths(&self) -> (PathBuf, PathBuf) {
        let random_suffix = random::<u64>();
        let source_path = self.work_dir.join(&format!("source-{}.c",
                                                      random_suffix));
        let exe_path = self.work_dir.join(&format!("exe-{}",
                                                   random_suffix))
                                    .with_extension(env::consts::EXE_EXTENSION);
        (source_path, exe_path)
    }

    /// Write a byte slice to a file, then attempt to compile it.
    ///
    /// This is not terribly useful, and is provided mostly for users who simply
    /// want to reuse a closure that was used to construct the `Probe`, as well
    /// as for convenience and testing of `probe-c-api` itself.
    pub fn check_compile(&self, source: &str) -> CommandResult {
        let (source_path, exe_path) = self.random_source_and_exe_paths();
        try!(write_to_new_file(&source_path, source));
        let compile_output = try!((*self.compile_to)(&source_path, &exe_path));
        try!(fs::remove_file(&source_path));
        // Remove the generated executable if it exists.
        match fs::remove_file(&exe_path) {
            Ok(..) => {}
            Err(error) => {
                if error.kind() != io::ErrorKind::NotFound {
                    return Err(error);
                }
            }
        }
        Ok(compile_output)
    }

    /// Write a byte slice to a file, then attempt to compile and run it.
    ///
    /// Like `check_compile`, this provides little value, but is available as a
    /// minor convenience.
    pub fn check_run(&self, source: &str) -> io::Result<CompileRunOutput> {
        let (source_path, exe_path) = self.random_source_and_exe_paths();
        try!(write_to_new_file(&source_path, source));
        let compile_output = try!((*self.compile_to)(&source_path, &exe_path));
        try!(fs::remove_file(&source_path));
        let run_output;
        if compile_output.status.success() {
            run_output = Some(try!((*self.run)(&exe_path)));
            try!(fs::remove_file(&exe_path));
        } else {
            run_output = None;
        }
        Ok(CompileRunOutput{
            compile_output: compile_output,
            run_output: run_output,
        })
    }

    /// Utility for various checks that use some simple code in `main`.
    fn main_source_template(&self, headers: Vec<&str>, main_body: &str)
                            -> String {
        let mut header_includes = String::new();
        for header in &self.headers {
            write!(&mut header_includes, "#include {}\n", header).unwrap();
        }
        for header in &headers {
            write!(&mut header_includes, "#include {}\n", header).unwrap();
        }
        format!("{}\n\
                 int main(int argc, char **argv) {{\n\
                 {}\n\
                 }}\n",
                header_includes,
                main_body)
    }

    /// Utility for code that simply prints a Rust constant, readable using
    /// `FromStr::from_str`, in `main`.
    fn run_to_get_rust_constant<T: FromStr>(&self,
                                            headers: Vec<&str>,
                                            main_body: &str)
                                            -> CProbeResult<T> {
        let source = self.main_source_template(headers, &main_body);
        let compile_run_output = try!(self.check_run(&source));
        let run_out_string = try!(compile_run_output.successful_run_output());
        // If the program produces invalid output, we don't really check what's
        // wrong with the output right now. Either the lossy UTF-8 conversion
        // will produce nonsense, or we will just fail to pick out a number
        // here.
        match FromStr::from_str(run_out_string.trim()) {
            Ok(size) => Ok(size),
            Err(..) => Err(OtherError("unexpected output from probe program"
                                      .to_string())),
        }
    }

    /// Get the size of a C type, in bytes.
    pub fn size_of(&self, type_: &str) -> CProbeResult<usize> {
        let headers: Vec<&str> = vec!["<stdio.h>"];
        let main_body = format!("printf(\"%zd\\n\", sizeof({}));\n\
                                 return 0;",
                                type_);
        self.run_to_get_rust_constant(headers, &main_body)
    }

    /// Get the alignment of a C type, in bytes.
    ///
    /// Note that this method depends on the compiler having implemented C11
    /// alignment facilities (specifically `stdalign.h` and `alignof`).
    pub fn align_of(&self, type_: &str) -> CProbeResult<usize> {
        let headers: Vec<&str> = vec!["<stdio.h>", "<stdalign.h>"];
        let main_body = format!("printf(\"%zd\\n\", alignof({}));\n\
                                 return 0;",
                                type_);
        self.run_to_get_rust_constant(headers, &main_body)
    }

    /// Check to see if a macro is defined.
    ///
    /// One obvious use for this is to check for macros that are intended to be
    /// used with `#ifdef`, e.g. macros that communicate configuration options
    /// originally used to build the library.
    ///
    /// A less obvious use is to check whether or not a constant or function has
    /// been implemented as a macro, for cases where this is not specified in
    /// the API documentation, or differs between library versions. In such
    /// cases, bindings may have to omit functionality provided by macros, or
    /// else implement such functionality via some special workaround.
    pub fn is_defined_macro(&self, token: &str) -> CProbeResult<bool> {
        let headers: Vec<&str> = vec!["<stdio.h>"];
        let main_body = format!("#ifdef {}\n\
                                 printf(\"true\");\n\
                                 #else\n\
                                 printf(\"false\");\n\
                                 #endif\n\
                                 return 0;",
                                token);
        self.run_to_get_rust_constant(headers, &main_body)
    }

    /// Check to see if an integer type is signed or unsigned.
    pub fn is_signed(&self, type_: &str) -> CProbeResult<bool> {
        let headers: Vec<&str> = vec!["<stdio.h>"];
        let main_body = format!("if ((({})-1) < 0) {{\n\
                                 printf(\"true\");\n\
                                 }} else {{\n\
                                 printf(\"false\");\n\
                                 }}\n\
                                 return 0;",
                                type_);
        self.run_to_get_rust_constant(headers, &main_body)
    }
}

// Little utility to cat something to a new file.
fn write_to_new_file(path: &Path, text: &str) -> io::Result<()> {
    // FIXME? Should we try putting in tests for each potential `try!` error?
    // It's hard to trigger them with Rust 1.0, since the standard library's
    // filesystem permission operations haven't been stabilized yet.
    let mut file = try!(fs::File::create(path));
    write!(&mut file, "{}", text)
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
            vec![],
            &env::temp_dir(),
            |source_path, exe_path| {
                Command::new("gcc").arg(source_path)
                                   .arg("-o").arg(exe_path)
                                   .output()
            },
            |exe_path| {
                Command::new(exe_path).output()
            },
        ).unwrap()
    }
}
