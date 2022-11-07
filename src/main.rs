mod track;

use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(about)]
/// A CLI for tracking time spent on different activities
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new activity to track
    New {
        /// Name for the activity
        #[arg(value_parser = parse_name)]
        name: String,
    },
    /// Set the activity that other commands should act on
    Set {
        /// Name of the activity
        #[arg(value_parser = parse_name)]
        name: String,
    },
    /// Delete an activity
    Delete {
        /// Name of the activity to delete
        #[arg(value_parser = parse_name)]
        name: String,
    },
    /// Display the name of the current activity
    Current,
    /// Display the names of all tracked activities
    All,
    /// Start tracking a session
    Start,
    /// End tracking of the ongoing session
    End {
        /// Optional notes
        #[arg(short, long, value_parser = parse_notes, default_value_t = String::new(), hide_default_value = true)]
        notes: String,
    },
    /// Cancel ongoing tracking of a session
    Cancel,
    /// Display details of the ongoing session
    Ongoing,
    /// Add a new session
    Add {
        /// Start date and time [dd/mm/yy HH:MM]
        #[arg(short, long, value_parser = parse_date_time)]
        start: NaiveDateTime,
        /// End date and time [dd/mm/yy HH:MM]
        #[arg(short, long, value_parser = parse_date_time)]
        end: NaiveDateTime,
        /// Optional notes
        #[arg(short, long, value_parser = parse_notes, default_value_t = String::new(), hide_default_value = true)]
        notes: String,
    },
    /// Edit a session
    ///
    /// Omit an argument to leave the corresponding value unchanged
    Edit {
        /// Position of the session to edit (either an index, or [last])
        #[arg(value_parser = parse_position)]
        position: Position,
        /// New start date and time [dd/mm/yy HH:MM]
        #[arg(short, long, value_parser = parse_date_time)]
        start: Option<NaiveDateTime>,
        /// New end date and time [dd/mm/yy HH:MM]
        #[arg(short, long, value_parser = parse_date_time)]
        end: Option<NaiveDateTime>,
        /// New notes
        #[arg(short, long, value_parser = parse_notes)]
        notes: Option<String>,
    },
    /// Remove a session
    Remove {
        /// Position of the session to remove (either an index, or [last])
        #[arg(value_parser = parse_position)]
        position: Position,
    },
    /// Display full session history, or sessions in a specified time range
    ///
    /// Omit <COMMAND> for full session history
    List {
        #[command(subcommand)]
        range_command: Option<RangeCommand>,
    },
    /// Display full session statistics, or session statistics in a specified time range
    ///
    /// Omit <COMMAND> for full session statistics
    Stats {
        #[command(subcommand)]
        range_command: Option<RangeCommand>,
    },
}

#[derive(Subcommand)]
enum RangeCommand {
    /// Sessions ranging between a specified amount of time in the past, and now
    Past {
        /// Number of weeks
        #[arg(short, long, default_value_t = 0, hide_default_value = true)]
        weeks: u32,
        /// Number of days
        #[arg(short, long, default_value_t = 0, hide_default_value = true)]
        days: u32,
        /// Number of hours
        #[arg(short = 'H', long, default_value_t = 0, hide_default_value = true)]
        hours: u32,
        /// Number of minutes
        #[arg(short = 'M', long, default_value_t = 0, hide_default_value = true)]
        minutes: u32,
    },
    /// Sessions ranging between a specified time, and now
    ///
    /// Omit <FROM> to start from the first session
    Since {
        /// Start date, or date and time, of the range ([dd/mm/yy] or [dd/mm/yy HH:MM])
        #[arg(value_parser = parse_bound)]
        from: Option<Bound>,
    },
    /// Sessions ranging between two specified times
    ///
    /// Omit <FROM> to start from the first session
    ///
    /// Omit <TO> to end at the last session
    Range {
        /// Start date, or date and time, of the range ([dd/mm/yy] or [dd/mm/yy HH:MM])
        #[arg(short, long, value_parser = parse_bound)]
        from: Option<Bound>,
        /// End date, or date and time, of the range ([dd/mm/yy] or [dd/mm/yy HH:MM])
        #[arg(short, long, value_parser = parse_bound)]
        to: Option<Bound>,
    },
    /// Sessions on a specific date
    On {
        /// The date [dd/mm/yy]
        #[arg(value_parser = parse_date)]
        date: NaiveDate,
    },
}

fn main() {
    if let Err(e) = run() {
        print!("{e}");
    }
}

fn run() -> Result<()> {
    let cli = Cli::try_parse()?;

    match cli.command {
        Command::New { name } => track::create(name),
        Command::Set { name } => track::set(name),
        Command::Delete { name } => track::delete(name),
        Command::Current => track::current(),
        Command::All => track::all(),
        Command::Start => track::start(),
        Command::End { notes } => track::end(notes),
        Command::Cancel => track::cancel(),
        Command::Ongoing => track::ongoing(),
        Command::Add { start, end, notes } => track::add(start, end, notes),
        Command::Edit {
            position,
            start,
            end,
            notes,
        } => track::edit(position, start, end, notes),
        Command::Remove { position } => track::remove(position),
        Command::List { range_command } => {
            let (from, to) = get_bounds(range_command);
            track::list(from, to)
        }
        Command::Stats { range_command } => {
            let (from, to) = get_bounds(range_command);
            track::stats(from, to)
        }
    }
}

fn get_bounds(command: Option<RangeCommand>) -> (Bound, Bound) {
    if let Some(command) = command {
        match command {
            RangeCommand::Past {
                weeks,
                days,
                hours,
                minutes,
            } => (
                Bound::Ago {
                    weeks,
                    days,
                    hours,
                    minutes,
                },
                Bound::Now,
            ),
            RangeCommand::Since { from } => (from.unwrap_or(Bound::None), Bound::Now),
            RangeCommand::Range { from, to } => {
                (from.unwrap_or(Bound::None), to.unwrap_or(Bound::None))
            }
            RangeCommand::On { date } => (Bound::Date(date), Bound::Date(date)),
        }
    } else {
        (Bound::None, Bound::None)
    }
}

fn parse_name(s: &str) -> Result<String, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("name must not be empty".to_string());
    }
    Ok(s.to_string())
}

fn parse_notes(s: &str) -> Result<String, String> {
    Ok(s.trim().to_string())
}

fn parse_position(s: &str) -> Result<Position, String> {
    if s == "last" {
        return Ok(Position::Last);
    } else if let Ok(i) = s.parse() {
        return Ok(Position::Index(i));
    } else {
        return Err("index must be either [last] or a non-negative integer".to_string());
    }
}

fn parse_date_time(s: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(s, "%d/%m/%y %R")
        .map_err(|_| "date and time must be in the form [dd/mm/yy HH:MM]".to_string())
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%d/%m/%y")
        .map_err(|_| "date must be in the form [dd/mm/yy]".to_string())
}

fn parse_bound(s: &str) -> Result<Bound, String> {
    if let Ok(date_time) = NaiveDateTime::parse_from_str(s, "%d/%m/%y %R") {
        return Ok(Bound::DateTime(date_time));
    } else if let Ok(date) = NaiveDate::parse_from_str(s, "%d/%m/%y") {
        return Ok(Bound::Date(date));
    }
    Err("must be in the form [dd/mm/yy] or [dd/mm/yy HH:MM]".to_string())
}

#[derive(Clone)]
pub enum Position {
    Index(usize),
    Last,
}

#[derive(Clone, Copy)]
pub enum Bound {
    None,
    Now,
    DateTime(NaiveDateTime),
    Date(NaiveDate),
    Ago {
        weeks: u32,
        days: u32,
        hours: u32,
        minutes: u32,
    },
}
