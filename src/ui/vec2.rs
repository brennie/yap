// Copyright 2020 Barret Rennie
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
//  option. This file may not be copied, modified, or distributed
// except according to those terms.

/// A two vector, representing sizes and positions in the terminal.
///
/// It is implicitly convertable from `(u16, u16)` because that is what crossterm uses for sizes.
#[derive(Clone, Copy, Default)]
pub struct Vec2 {
    pub x: usize,
    pub y: usize,
}

impl From<(u16, u16)> for Vec2 {
    fn from((x, y): (u16, u16)) -> Self {
        Vec2 {
            x: x as usize,
            y: y as usize,
        }
    }
}
