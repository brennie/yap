// Copyright 2020 Barret Rennie
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
//  option. This file may not be copied, modified, or distributed
// except according to those terms.

mod document;
mod vec2;

use std::io::{self, StdoutLock, Write};

use crossterm::event::{Event, EventStream, KeyCode};
use crossterm::style::{self, Attribute};
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode, ClearType};
use crossterm::{cursor, execute};
use futures::prelude::*;
use futures::select;
use futures::stream::TryStreamExt;
use tokio::io::{AsyncRead, BufReader};
use tokio::prelude::*;

use crate::ui::document::{DocumentView, FileDocument, HelpDocument};
use crate::ui::vec2::Vec2;

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

/// The current yap UI state.
struct UiState<'a> {
    /// The document being viewed.
    document_view: DocumentView<FileDocument>,

    help_view: Option<DocumentView<HelpDocument>>,

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
            document_view: DocumentView::new(
                FileDocument::new(size.y - 2),
                Vec2 {
                    x: size.x - 2,
                    y: size.y - 2,
                },
            ),
            help_view: None,
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
                KeyCode::Char('q') | KeyCode::Char('Q') => self.quit()?,
                KeyCode::Char('h') => self.pan_left()?,
                KeyCode::Char('j') => self.scroll_down()?,
                KeyCode::Char('k') => self.scroll_up()?,
                KeyCode::Char('l') => self.pan_right()?,
                KeyCode::Char(' ') | KeyCode::PageDown => self.next_page()?,
                KeyCode::Char('?') => self.show_help()?,
                KeyCode::PageUp => self.prev_page()?,
                _ => {}
            },
        }

        Ok(())
    }

    /// Handle a line being buffered in from the input stream.
    ///
    /// The line will be displayed if there is room to draw it.
    pub fn handle_line(&mut self, line: String) -> crossterm::Result<()> {
        let index = self.document_view.document().push_line(line);
        if self.help_view.is_none()
            && self
                .document_view
                .queue_line_if_visible(&mut self.stdout, index)?
        {
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
        self.document_view.resize(Vec2 {
            x: new_size.x - 2,
            y: new_size.y - 2,
        });
        if let Some(help_view) = self.help_view.as_mut() {
            help_view.resize(Vec2 {
                x: new_size.x - 2,
                y: new_size.y - 2,
            });

            help_view.redraw(&mut self.stdout)
        } else {
            self.document_view.redraw(&mut self.stdout)
        }
    }

    /// Draw the status bar.
    ///
    /// Note: this method does not reposition the cursor after moving it to the status line.
    fn draw_status_bar(&mut self) -> crossterm::Result<()> {
        execute!(
            self.stdout,
            cursor::MoveTo(0, (self.size.y - 1) as u16),
            style::SetAttribute(Attribute::Reverse),
            style::Print("[yap] q to exit, ? for help"),
            style::SetAttribute(Attribute::NoReverse),
        )
    }

    fn show_help(&mut self) -> crossterm::Result<()> {
        if self.help_view.is_none() {
            let help_view = DocumentView::new(
                HelpDocument,
                Vec2 {
                    x: self.size.x - 2,
                    y: self.size.y - 2,
                },
            );

            execute!(self.stdout, terminal::Clear(ClearType::All))?;
            self.draw_status_bar()?;
            help_view.redraw(&mut self.stdout)?;

            self.help_view = Some(help_view);
        }

        Ok(())
    }

    fn quit(&mut self) -> crossterm::Result<()> {
        if self.help_view.is_some() {
            self.help_view = None;
            execute!(self.stdout, terminal::Clear(ClearType::All))?;
            self.draw_status_bar()?;
            self.document_view.redraw(&mut self.stdout)
        } else {
            self.should_exit = true;
            Ok(())
        }
    }
}

macro_rules! impl_document_view_methods {
    (
        $(#[doc = $doc:expr])+
        $method:ident,
        $($rest:tt)*
    ) => {
        $(#[doc = $doc])+
        pub fn $method(&mut self) -> crossterm::Result<()> {
            if let Some(help_view) = self.help_view.as_mut() {
                help_view.$method(&mut self.stdout)
            } else {
                self.document_view.$method(&mut self.stdout)
            }
        }

        impl_document_view_methods!($($rest)*);
    };

    () => {};
}

impl<'a> UiState<'a> {
    impl_document_view_methods! {
        /// Pan left by one column if we are not at the first column of the document.
        pan_left,

        /// Scroll down by one line if there is at least one more line of text off-screen.
        scroll_down,

        /// Scroll up by one line if we are not at the top of the document.
        scroll_up,

        /// Pan right by one column if there is at least one more column of text off-screen.
        pan_right,

        /// Scroll the doucment up by up to half the height of the terminal if we are not at the top
        /// of the document.
        prev_page,

        /// Scroll the document down by up to half the height of the terminal if there is more
        /// document to view.
        next_page,
    }
}
