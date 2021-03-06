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

use probe_c_api::Probe;

#[test]
fn check_compile_pass() {
    let probe = <Probe>::default();
    assert!(probe.check_compile("int main() { return 0; }")
                 .unwrap().status.success());
}

#[test]
fn check_compile_fail() {
    let probe = <Probe>::default();
    assert!(!probe.check_compile("ain't it a C progarm, bub!")
                  .unwrap().status.success());
}
