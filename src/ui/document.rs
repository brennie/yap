// Copyright 2020 Barret Rennie
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
//  option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::cmp::{max, min};
use std::io::{StdoutLock, Write};
use std::ops::Range;

use crossterm::terminal::{self, ClearType};
use crossterm::{cursor, queue, style};

use crate::ui::Vec2;

pub struct Document {
    /// The lines of the document.
    lines: Vec<String>,

    /// The length of the longest line.
    max_line_len: usize,

    /// The offset into `lines.`
    offset: Vec2,

    /// The size of the display region.
    size: Vec2,
}

impl Document {
    pub fn new(size: Vec2) -> Self {
        Document {
            lines: Vec::with_capacity(size.y),
            max_line_len: 0,
            offset: Vec2::default(),
            size: size,
        }
    }

    pub fn handle_line(&mut self, line: String) -> Option<usize> {
        let index = self.lines.len();
        self.max_line_len = max(self.max_line_len, line.chars().count());
        self.lines.push(line);

        if self.visible_pane_rows().contains(&index) {
            Some(index)
        } else {
            None
        }
    }

    pub fn resize(&mut self, new_size: Vec2) {
        self.size = new_size;
    }

    /// Pan left by one column if we are not at the first column of the document.
    pub fn pan_left<'a>(&mut self, stdout: &mut StdoutLock<'a>) -> crossterm::Result<()>
    {
        if self.offset.x > 0 {
            self.offset.x -= 1;
            self.redraw(stdout)?;
        }

        Ok(())
    }

    /// Scroll down by one line if there is at least one more line of text off-screen.
    pub fn scroll_down<'a>(&mut self, stdout: &mut StdoutLock<'a>) -> crossterm::Result<()>
    {
        if self.lines.len() > self.offset.y + self.size.y {
            self.offset.y += 1;
            self.redraw(stdout)?;
        }

        Ok(())
    }

    /// Scroll up by one line if we are not at the top of the document.
    pub fn scroll_up<'a>(&mut self, stdout: &mut StdoutLock<'a>) -> crossterm::Result<()>
    {
        if self.offset.y > 0 {
            self.offset.y -= 1;
            self.redraw(stdout)?;
        }

        Ok(())
    }

    /// Pan right by one column if there is at least one more column of text off-screen.
    pub fn pan_right<'a>(&mut self, stdout: &mut StdoutLock<'a>) -> crossterm::Result<()>
    {
        if self.max_line_len > self.offset.x + self.size.x {
            self.offset.x += 1;
            self.redraw(stdout)?;
        }

        Ok(())
    }

    /// Scroll the doucment up by up to half the height of the terminal if we are not at the top of
    /// the document.
    pub fn prev_page<'a>(&mut self, stdout: &mut StdoutLock<'a>) -> crossterm::Result<()>
    {
        let page_size = min(self.size.y / 2, self.offset.y);
        if self.offset.y > 0 {
            self.offset.y -= page_size;
            self.redraw(stdout)?;
        }

        Ok(())
    }

    /// Scroll the document down by up to half the height of the terminal if there is more document
    /// to view.
    pub fn next_page<'a>(&mut self, stdout: &mut StdoutLock<'a>) -> crossterm::Result<()>
    {
        let page_size = self.size.y / 2;

        if self.lines.len() >= self.size.y + self.offset.y + page_size {
            // Scroll down by an entire page if we can.
            self.offset.y += page_size;
            self.redraw(stdout)?;
        } else if self.lines.len() > self.size.y + self.offset.y {
            // Otherwise, if we are not at the end of the document, then scroll to the end.
            self.offset.y = self.lines.len() - self.size.y;
            self.redraw(stdout)?;
        }

        Ok(())
    }

    pub fn queue_line<'a>(&self, stdout: &mut StdoutLock<'a>, index: usize) -> crossterm::Result<()>
    {
        let line = &self.lines[index];
        let mut char_indices = line.char_indices().map(|(idx, _)| idx);

        // Find the index of the character as position `self.offset.x`. If no character exists, then
        // this line is too short to display on screen, so we can just clear the line.
        let start = match char_indices.nth(self.offset.x) {
            Some(char_index) => char_index,
            None => {
                return queue!(
                    stdout,
                    terminal::Clear(ClearType::UntilNewLine),
                    cursor::MoveToNextLine(1),
                );
            }
        };

        // If the line would be too long to display from `start`, find the index of the character
        // one past the screen. Otherwise, we can default to the string length.
        let end = char_indices.nth(self.size.x).unwrap_or(line.len());

        queue!(
            stdout,
            style::Print(&line[start..end]),
            terminal::Clear(ClearType::UntilNewLine),
            cursor::MoveToNextLine(1),
        )
    }

    /// Redraw the document to the screen.
    pub fn redraw<'a>(&self, stdout: &mut StdoutLock<'a>) -> crossterm::Result<()>
    {
        queue!(stdout, cursor::MoveTo(0, 0))?;

        for y in self.visible_lines() {
            self.queue_line(stdout, y)?;
        }

        stdout.flush()?;

        Ok(())
    }

    /// The range of lines in the document that are visible.
    fn visible_lines(&self) -> Range<usize> {
        self.offset.y..min(self.offset.y + self.size.y, self.lines.len())
    }

    /// The range of lines that would be visible, unbounded by the size of the document.
    fn visible_pane_rows(&self) -> Range<usize> {
        self.offset.y..self.offset.y + self.size.y
    }
}
