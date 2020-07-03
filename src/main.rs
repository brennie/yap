// Copyright 2020 Barret Rennie
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
//  option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::path::PathBuf;

use structopt::StructOpt;

/// yap yet another pager
///
/// yap is a program similar to less or more that allows for reading multi-page documents in your
/// terminal. yap's keybindings are inspired by vi.
#[derive(StructOpt)]
struct Options {
    /// The file to read.
    /// If not provided and standard input is not a TTY, yap will read from standard input instead.
    file: Option<PathBuf>,
}

fn main() {
    Options::from_args();
}
