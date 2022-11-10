mod track;

use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
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
    /// Cancel tracking of the ongoing session
    Cancel,
    /// Display details of the ongoing session
    Ongoing,
    #[clap(
        about = "Add a new session",
        long_about = ADD_ABOUT
    )]
    Add {
        /// Session start
        #[arg(short, long, value_parser = parse_ts)]
        start: TimeSpecifier,
        /// Session end
        #[arg(short, long, value_parser = parse_ts)]
        end: TimeSpecifier,
        /// Optional notes
        #[arg(short, long, value_parser = parse_notes, default_value_t = String::new(), hide_default_value = true)]
        notes: String,
    },
    #[clap(
        about = "Edit a session",
        long_about = EDIT_ABOUT
    )]
    Edit {
        /// Position of the session to edit
        #[arg(value_parser = parse_position)]
        position: Position,
        /// New session start
        #[arg(short, long, value_parser = parse_ts)]
        start: Option<TimeSpecifier>,
        /// New session end
        #[arg(short, long, value_parser = parse_ts)]
        end: Option<TimeSpecifier>,
        /// New notes
        #[arg(short, long, value_parser = parse_notes)]
        notes: Option<String>,
    },
    #[clap(
        about = "Remove a session",
        long_about = REMOVE_ABOUT)]
    Remove {
        /// Position of the session to remove
        #[arg(value_parser = parse_position)]
        position: Position,
    },
    #[clap(
        about = "Display full session history, or sessions in a specific time range",
        long_about = LIST_ABOUT)]
    List {
        #[command(subcommand)]
        range_command: Option<RangeCommand>,
    },
    #[clap(
        about = "Display full session statistics, or session statistics in a specific time range",
        long_about = STATS_ABOUT)]
    Stats {
        #[command(subcommand)]
        range_command: Option<RangeCommand>,
    },
}

#[derive(Subcommand)]
enum RangeCommand {
    #[clap(about = "Sessions ranging between a specific amount of time in the past, and now",
    long_about = PAST_ABOUT)]
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
    #[clap(about = "Sessions ranging between a specific time, and now", long_about = SINCE_ABOUT)]
    Since {
        /// Start of the range
        #[arg(value_parser = parse_ts)]
        start: Option<TimeSpecifier>,
    },
    #[clap(about = "Sessions ranging between two specific times", long_about = RANGE_ABOUT)]
    Range {
        /// Start of the range
        #[arg(short, long, value_parser = parse_ts)]
        start: Option<TimeSpecifier>,
        /// End of the range
        #[arg(short, long, value_parser = parse_ts)]
        end: Option<TimeSpecifier>,
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
            let (start, end) = get_bounds(range_command);
            track::list(start, end)
        }
        Command::Stats { range_command } => {
            let (start, end) = get_bounds(range_command);
            track::stats(start, end)
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
            RangeCommand::Since { start } => (map_ts(start), Bound::Now),
            RangeCommand::Range { start, end } => (map_ts(start), map_ts(end)),
            RangeCommand::On { date } => (
                Bound::TimeSpecifier(TimeSpecifier::Date(date)),
                Bound::TimeSpecifier(TimeSpecifier::Date(date)),
            ),
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
        Ok(Position::Last)
    } else if let Ok(i) = s.parse() {
        Ok(Position::Index(i))
    } else {
        Err("index must be either [last] or a non-negative integer".to_string())
    }
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%d/%m/%y")
        .map_err(|_| "date must be in the form [dd/mm/yy]".to_string())
}

fn parse_ts(s: &str) -> Result<TimeSpecifier, String> {
    if let Ok(date_time) = NaiveDateTime::parse_from_str(s, "%d/%m/%y %R") {
        return Ok(TimeSpecifier::DateTime(date_time));
    } else if let Ok(date) = NaiveDate::parse_from_str(s, "%d/%m/%y") {
        return Ok(TimeSpecifier::Date(date));
    } else if let Ok(time) = NaiveTime::parse_from_str(s, "%R") {
        return Ok(TimeSpecifier::Time(time));
    }
    Err("must be in the form [dd/mm/yy] or [HH:MM] or [dd/mm/yy HH:MM]".to_string())
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
    TimeSpecifier(TimeSpecifier),
    Ago {
        weeks: u32,
        days: u32,
        hours: u32,
        minutes: u32,
    },
}

#[derive(Clone, Copy)]
pub enum TimeSpecifier {
    DateTime(NaiveDateTime),
    Date(NaiveDate),
    Time(NaiveTime),
}

fn map_ts(ts: Option<TimeSpecifier>) -> Bound {
    if let Some(ts) = ts {
        Bound::TimeSpecifier(ts)
    } else {
        Bound::None
    }
}

const ADD_ABOUT: &str = "Add a new session
    
<START>: [dd/mm/yy HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 on dd/mm/yy
         [HH:MM]          - HH:MM on today's date

<END>:   [dd/mm/yy HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 the day after dd/mm/yy
         [HH:MM]          - HH:MM on <START>'s date";

const EDIT_ABOUT: &str = "Edit a session

<POSITION>: [index]          - index of the session, as shown in track list
            \"last\"           - last recorded session
    
<START>:    [dd/mm/yy HH:MM] - HH:MM on dd/mm/yy
            [dd/mm/yy]       - 00:00 on dd/mm/yy
            [HH:MM]          - HH:MM on today's date
            omitted          - leave unchanged

<END>:      [dd/mm/yy HH:MM] - HH:MM on dd/mm/yy
            [dd/mm/yy]       - 00:00 the day after dd/mm/yy
            [HH:MM]          - HH:MM on <START>'s date
            omitted          - leave unchanged
            
Omit <NOTES> to leave notes unchanged";

const REMOVE_ABOUT: &str = "Remove a session

<POSITION>: [index]          - index of the session, as shown in track list
            \"last\"           - last recorded session";

const LIST_ABOUT: &str = "Display full session history, or sessions in a specific time range

Omit <COMMAND> for full session history";

const STATS_ABOUT: &str =
    "Display full session statistics, or sessions statistics in a specific time range

Omit <COMMAND> for full session statistics";

const PAST_ABOUT: &str = "Sessions ranging between a specific amount of time in the past, and now

Omit all arguments to start from the first session";

const SINCE_ABOUT: &str = "Sessions ranging between a specific time, and now

<START>: [dd/mm/yy HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 on dd/mm/yy
         [HH:MM]          - HH:MM on today's date
         omitted          - start of first recorded session";

const RANGE_ABOUT: &str = "Sessions ranging between two specific times

<START>: [dd/mm/yy HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 on dd/mm/yy
         [HH:MM]          - HH:MM on today's date
         omitted          - start of first recorded session
         
<END>:   [dd/mm/yy HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 the day after dd/mm/yy
         [HH:MM]          - HH:MM on <START>'s date
         omitted          - end of last recorded session";
