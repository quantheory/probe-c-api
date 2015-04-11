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

use std::default::Default;

use probe_c_api::Probe;

#[test]
fn try_compile_pass() {
    let probe = <Probe>::default();
    assert!(probe.try_compile("int main() { return 0; }".as_bytes()).unwrap()
                 .status.success());
}

#[test]
fn try_compile_fail() {
    let probe = <Probe>::default();
    assert!(!probe.try_compile("ain't it a C progarm, bub!".as_bytes()).unwrap()
                  .status.success());
}
