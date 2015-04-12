// Copyright 2015 Sean Patrick Santos
//
// Licensed under the Apache License, Version 2.0.
// License available at:
// http://www.apache.org/licenses/LICENSE-2.0
//
// Copyright and license information should be present in the top
// level directory of this package.

extern crate probe_c_api;

use std::default::Default;

use probe_c_api::Probe;

#[test]
fn sizeof_char() {
    let probe = <Probe>::default();
    let char_size = probe.check_sizeof("char".as_bytes()).unwrap();
    assert_eq!(1, char_size);
}
