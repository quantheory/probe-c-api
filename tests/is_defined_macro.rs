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
fn check_macro_definition() {
    let probe = <Probe>::default();
    assert!(probe.is_defined_macro("__STDC__").unwrap());
    assert!(!probe.is_defined_macro("THISSHOULDNTBEDEFINED").unwrap());
}
