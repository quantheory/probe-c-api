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

use probe_c_api::Probe;

#[test]
fn check_run_pass() {
    let probe = <Probe>::default();
    assert!(probe.check_run("int main() { return 0; }")
                 .unwrap().run_output.unwrap().status.success());
}

#[test]
fn check_run_fail() {
    let probe = <Probe>::default();
    assert!(!probe.check_run("int main() { return 1; }")
                  .unwrap().run_output.unwrap().status.success());
}

#[test]
fn check_run_compile_fail() {
    let probe = <Probe>::default();
    let compile_run_output =
        probe.check_run("I don't think this is a C program.").unwrap();
    assert!(!compile_run_output.compile_output.status.success());
    assert!(compile_run_output.run_output.is_none());
}
