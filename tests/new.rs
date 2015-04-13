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
use std::fs;
use std::io::Write;
use std::process::Command;

use probe_c_api::Probe;
use probe_c_api::NewProbeError::*;

#[test]
fn new_probe_checks_directory() {
    let file_path = env::temp_dir().join("foo.txt");
    {
        let mut f = fs::File::create(&file_path).unwrap();
        f.write_all("bar\n".as_bytes()).unwrap();
    }
    let new_probe_result = Probe::new(
        vec![],
        &file_path,
        |_, _| { Command::new(":").output() },
        |_| { Command::new(":").output() },
    );
    assert!(match new_probe_result {
        Err(WorkDirNotADirectory(..)) => true,
        _ => false,
    });
}

#[test]
fn new_probe_errors_on_inaccessible_metadata() {
    let fake_path = env::temp_dir().join("not_a_real_directory");
    let new_probe_result = Probe::new(
        vec![],
        &fake_path,
        |_, _| { Command::new(":").output() },
        |_| { Command::new(":").output() },
    );
    assert!(match new_probe_result {
        Err(WorkDirMetadataInaccessible(..)) => true,
        _ => false,
    });
}
