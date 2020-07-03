// Copyright 2020 Barret Rennie
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
//  option. This file may not be copied, modified, or distributed
// except according to those terms.

mod ui;

use std::path::PathBuf;
use std::process::exit;

use anyhow::{anyhow, Context};
use crossterm::tty::IsTty;
use structopt::StructOpt;
use tokio::fs::File;
use tokio::io::stdin;

use crate::ui::ui;

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

#[tokio::main]
async fn run(options: Options) -> anyhow::Result<()> {
    if let Some(path) = options.file {
        let f = File::open(&path)
            .await
            .context(format!("Could not open `{}' for reading", path.display()))?;
        ui(f).await?;
    } else if !stdin().is_tty() {
        ui(stdin()).await?;
    } else {
        return Err(anyhow!("yap: requires file or pipe"));
    }

    Ok(())
}

fn main() {
    let options = Options::from_args();

    exit(match run(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("yap: {:#}", e);
            1
        }
    });
}
