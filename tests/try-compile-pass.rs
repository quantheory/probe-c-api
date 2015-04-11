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
use std::str::from_utf8;

use probe_c_api::Probe;

#[test]
fn try_compile_pass() {
    // FIXME! Pretty *nix-centric.
    let probe = Probe::new(
        Path::new(&env::var_os("TMPDIR").unwrap_or(OsString::from("/tmp"))),
        |source_path, program_path| {
            Command::new("gcc").arg("-c").arg(source_path)
                               .arg("-o").arg(program_path)
                               .output()
        },
        |program_path| {
            Command::new(program_path).output()
        },
    );
    println!("{:?}",
             from_utf8(&probe.try_compile("int main() { return 0; }".as_bytes()).unwrap()
                             .stderr));
    assert!(probe.try_compile("int main() { return 0; }".as_bytes()).unwrap()
                 .status.success());
}
