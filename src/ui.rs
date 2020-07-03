// Copyright 2020 Barret Rennie
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
//  option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::cmp::min;
use std::io::{self, StdoutLock, Write};
use std::ops::Range;

use crossterm::event::{Event, EventStream, KeyCode};
use crossterm::style::{self, Attribute};
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode, ClearType};
use crossterm::{cursor, execute, queue};
use futures::prelude::*;
use futures::select;
use futures::stream::TryStreamExt;
use tokio::io::{AsyncRead, BufReader};
use tokio::prelude::*;

/// Run the yap UI.
///
/// The `input` arugment is the stream (either stdin or a file) that will be displayed.
pub async fn ui<R>(input: R) -> crossterm::Result<()>
where
    R: AsyncRead + Unpin,
{
    let stdout = io::stdout();
    let mut input = BufReader::new(input).lines();
    let mut events = EventStream::new().fuse();

    let mut ui_state = UiState::new(stdout.lock(), terminal::size()?.into());

    ui_state.initialize_terminal()?;

    loop {
        select! {
            event = events.try_next() => ui_state.handle_event(event?)?,
            line = input.next_line().fuse() => {
                if let Some(line) = line? {
                    ui_state.handle_line(line)?;
                }
            }
        }

        if ui_state.should_exit() {
            break;
        }
    }

    ui_state.finalize_terminal()?;

    Ok(())
}

/// A two vector, representing sizes and positions in the terminal.
///
/// It is implicitly convertable from `(u16, u16)` because that is what crossterm uses for sizes.
#[derive(Clone, Copy)]
pub struct Vec2 {
    x: usize,
    y: usize,
}

impl From<(u16, u16)> for Vec2 {
    fn from((x, y): (u16, u16)) -> Self {
        Vec2 {
            x: x as usize,
            y: y as usize,
        }
    }
}

/// The current yap UI state.
struct UiState<'a> {
    /// The lines of the document.
    document: Vec<String>,

    /// Whether or not yap should exit.
    should_exit: bool,

    /// The current size of the terminal.
    size: Vec2,

    /// A lock on the output handle.
    stdout: StdoutLock<'a>,
}

impl<'a> UiState<'a> {
    /// Create a new UiState.
    pub fn new(stdout: StdoutLock<'a>, size: Vec2) -> Self {
        UiState {
            document: Vec::with_capacity(size.y),
            should_exit: false,
            size,
            stdout,
        }
    }

    /// Whether or not yap should exit.
    pub fn should_exit(&self) -> bool {
        self.should_exit
    }

    /// Initialize the terminal.
    ///
    /// This method will enable raw mode on the tty, switch to the alternate screen, and hide the
    /// cursor.
    pub fn initialize_terminal(&mut self) -> crossterm::Result<()> {
        enable_raw_mode()?;
        execute!(self.stdout, terminal::EnterAlternateScreen, cursor::Hide)?;
        Ok(())
    }

    /// Finalize the terminal, returning its state to normal.
    ///
    /// This method undoes the transforms from [`initialize_terminal()`][initialize_terminal].
    ///
    /// [initialize_terminal]: struct.UiState.html#method.initialize_terminal
    pub fn finalize_terminal(&mut self) -> crossterm::Result<()> {
        execute!(self.stdout, terminal::LeaveAlternateScreen, cursor::Show)?;
        disable_raw_mode()?;
        Ok(())
    }

    /// Handle an event from crossterm.
    ///
    /// If `event` is `None`, the next call to [`should_exit()`][should_exit] will return true.
    /// Otherwise, the event is handled and the UI is updated accordingly.
    pub fn handle_event(&mut self, event: Option<Event>) -> crossterm::Result<()> {
        let event = match event {
            Some(event) => event,
            None => {
                self.should_exit = true;
                return Ok(());
            }
        };

        match event {
            Event::Mouse(..) => unreachable!("yap does not have mouse support"),
            Event::Resize(x, y) => self.handle_resize((x, y).into())?,
            Event::Key(key) => match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => self.should_exit = true,
                _ => {}
            },
        }

        Ok(())
    }

    /// Handle a line being buffered in from the input stream.
    ///
    /// The line will be displayed if there is room to draw it.
    pub fn handle_line(&mut self, line: String) -> crossterm::Result<()> {
        let index = self.document.len();
        self.document.push(line);

        if self.document_pane().contains(&index) {
            self.queue_line(index)?;
            self.stdout.flush()?;
        }

        Ok(())
    }

    /// Handle a resize event.
    ///
    /// The entire screen will be cleared and re-drawn.
    fn handle_resize(&mut self, new_size: Vec2) -> crossterm::Result<()> {
        self.size = new_size;
        execute!(self.stdout, terminal::Clear(ClearType::All))?;
        self.draw_status_bar()?;
        self.redraw_document()
    }

    /// Draw the status bar.
    ///
    /// Note: this method does not reposition the cursor after moving it to the status line.
    fn draw_status_bar(&mut self) -> crossterm::Result<()> {
        execute!(
            self.stdout,
            cursor::MoveTo(0, (self.size.y - 1) as u16),
            style::SetAttribute(Attribute::Reverse),
            style::Print("[yap] q to exit"),
            style::SetAttribute(Attribute::NoReverse),
        )
    }

    /// Redraw the document to the screen.
    fn redraw_document(&mut self) -> crossterm::Result<()> {
        queue!(self.stdout, cursor::MoveTo(0, 0))?;

        for y in self.visible_document() {
            self.queue_line(y)?;
        }

        self.stdout.flush()?;

        Ok(())
    }

    /// Queue a line to be drawn.
    ///
    /// After queueing lines, they must be flushed with `self.stdout.flush()`.
    fn queue_line(&mut self, index: usize) -> crossterm::Result<()> {
        queue!(
            self.stdout,
            style::Print(&self.document[index]),
            terminal::Clear(ClearType::UntilNewLine),
            cursor::MoveToNextLine(1),
        )
    }

    /// Return the range of terminal lines that are in the document pane.
    fn document_pane(&self) -> Range<usize> {
        0..self.size.y - 2
    }

    /// Return the indicies of the document that are visible.
    fn visible_document(&self) -> Range<usize> {
        0..min(self.size.y - 2, self.document.len())
    }
}