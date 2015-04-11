// Copyright 2015 Sean Patrick Santos
//
// Licensed under the Apache License, Version 2.0.
// License available at:
// http://www.apache.org/licenses/LICENSE-2.0
//
// Copyright and license information should be present in the top
// level directory of this package.

#![crate_type="bin"]

extern crate probe_c_api;

use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use probe_c_api::Probe;

#[test]
fn try_compile_pass() {
    let probe = Probe::new(
        &env::temp_dir(),
        |source_path, program_path| {
            Command::new("gcc").arg("-c").arg(source_path)
                               .arg("-o").arg(program_path)
                               .output()
        },
        |program_path| {
            Command::new(program_path).output()
        },
    );
    assert!(probe.try_compile("int main() { return 0; }".as_bytes()).unwrap()
                 .status.success());
}

#[test]
fn try_compile_fail() {
    let probe = Probe::new(
        &env::temp_dir(),
        |source_path, program_path| {
            Command::new("gcc").arg("-c").arg(source_path)
                               .arg("-o").arg(program_path)
                               .output()
        },
        |program_path| {
            Command::new(program_path).output()
        },
    );
    assert!(!probe.try_compile("ain't it a C progarm, bub!".as_bytes()).unwrap()
                  .status.success());
}
