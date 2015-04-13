// Copyright 2015 Sean Patrick Santos
//
// Licensed under the Apache License, Version 2.0.
// A copy of this license is available at:
// http://www.apache.org/licenses/LICENSE-2.0
//
// Copyright and license information should also be present in the top
// level directory of this package.

extern crate probe_c_api;

use std::default::Default;
use std::env;
use std::process::Command;

use probe_c_api::{CProbeError, Probe};

#[test]
fn sizeof_char() {
    let probe = <Probe>::default();
    let char_size = probe.size_of("char").unwrap();
    assert_eq!(1, char_size);
}

#[test]
fn sizeof_compilation_error() {
    let probe = <Probe>::default();
    let error = probe.size_of("><").unwrap_err();
    assert!(match error {
        CProbeError::CompileError(..) => true,
        _ => false,
    });
}

#[test]
fn sizeof_type_in_header() {
    let probe = Probe::new(
        vec!["<inttypes.h>".to_string()],
        &env::temp_dir(),
        |source_path, exe_path| {
            Command::new("gcc").arg(source_path)
                               .arg("-o").arg(exe_path)
                               .output()
        },
        |exe_path| {
            Command::new(exe_path).output()
        },
    ).unwrap();
    let i32_size = probe.size_of("int32_t").unwrap();
    assert_eq!(4, i32_size);
}
