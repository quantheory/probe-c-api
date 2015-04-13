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
