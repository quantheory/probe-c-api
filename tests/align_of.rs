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
#[cfg_attr(not(test_alignof), ignore)]
fn alignof_char() {
    let probe = <Probe>::default();
    let char_align = probe.align_of("char").unwrap();
    assert_eq!(1, char_align);
}
