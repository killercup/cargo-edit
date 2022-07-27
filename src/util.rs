use std::io::Write;

pub use termcolor::{Color, ColorChoice};
use termcolor::{ColorSpec, StandardStream, WriteColor};

use crate::{CargoResult, Context};

/// Whether to color logged output
pub fn colorize_stderr() -> ColorChoice {
    if concolor_control::get(concolor_control::Stream::Stderr).color() {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    }
}

/// Print a message with a colored title in the style of Cargo shell messages.
pub fn shell_print(status: &str, message: &str, color: Color, justified: bool) -> CargoResult<()> {
    let color_choice = colorize_stderr();
    let mut output = StandardStream::stderr(color_choice);

    output.set_color(ColorSpec::new().set_fg(Some(color)).set_bold(true))?;
    if justified {
        write!(output, "{status:>12}")?;
    } else {
        write!(output, "{}", status)?;
        output.set_color(ColorSpec::new().set_bold(true))?;
        write!(output, ":")?;
    }
    output.reset()?;

    writeln!(output, " {message}").with_context(|| "Failed to write message")?;

    Ok(())
}

/// Print a styled action message.
pub fn shell_status(action: &str, message: &str) -> CargoResult<()> {
    shell_print(action, message, Color::Green, true)
}

/// Print a styled warning message.
pub fn shell_warn(message: &str) -> CargoResult<()> {
    shell_print("warning", message, Color::Yellow, false)
}

/// Print a styled warning message.
pub fn shell_note(message: &str) -> CargoResult<()> {
    shell_print("note", message, Color::Cyan, false)
}

/// Print a part of a line with formatting
pub fn shell_write_stderr(fragment: impl std::fmt::Display, spec: &ColorSpec) -> CargoResult<()> {
    let color_choice = colorize_stderr();
    let mut output = StandardStream::stderr(color_choice);

    output.set_color(spec)?;
    write!(output, "{}", fragment)?;
    output.reset()?;
    Ok(())
}
