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
#[cfg_attr(not(test_alignof), ignore)]
fn alignof_char() {
    let probe = <Probe>::default();
    let char_align = probe.align_of("char").unwrap();
    assert_eq!(1, char_align);
}
