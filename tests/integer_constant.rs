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

use std::env;
use std::process::Command;

use probe_c_api::Probe;

fn new_constant_probe() -> Probe<'static> {
    Probe::new(
        vec!["\"tests/test_constants.h\"".into()],
        &env::temp_dir(),
        |source_path, exe_path| {
            Command::new("gcc").arg(source_path)
                               .arg(format!("-I{}", env!("CARGO_MANIFEST_DIR")))
                               .arg("-o").arg(exe_path)
                               .output()
        },
        |exe_path| {
            Command::new(exe_path).output()
        },
    ).unwrap()
}

#[test]
fn signed_integer_constant() {
    let probe = new_constant_probe();
    assert_eq!(-1, probe.signed_integer_constant("negative_one").unwrap());
}

fn signed_integer_constant_from_macro() {
    let probe = new_constant_probe();
    assert_eq!(-1, probe.signed_integer_constant("NEGATIVE_ONE").unwrap());
}
