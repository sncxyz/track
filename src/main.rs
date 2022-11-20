mod track;

use anyhow::Result;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use clap::{Parser, Subcommand};
use track::{commands, Absolute, Bound, Position};

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
    /// Rename an activity
    Rename {
        /// Name of the activity to rename
        #[arg(value_parser = parse_name)]
        from: String,
        /// New name
        #[arg(value_parser = parse_name)]
        to: String,
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
        #[arg(short, value_parser = parse_notes, default_value_t = String::new(), hide_default_value = true)]
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
        #[arg(value_parser = parse_abs)]
        start: Absolute,
        /// Session end
        #[arg(value_parser = parse_abs)]
        end: Absolute,
        /// Optional notes
        #[arg(short, value_parser = parse_notes, default_value_t = String::new(), hide_default_value = true)]
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
        #[arg(short, value_parser = parse_abs)]
        start: Option<Absolute>,
        /// New session end
        #[arg(short, value_parser = parse_abs)]
        end: Option<Absolute>,
        /// New notes
        #[arg(short, value_parser = parse_notes)]
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
        #[arg(short, default_value_t = 0, hide_default_value = true)]
        weeks: u32,
        /// Number of days
        #[arg(short, default_value_t = 0, hide_default_value = true)]
        days: u32,
        /// Number of hours
        #[arg(short = 'H', default_value_t = 0, hide_default_value = true)]
        hours: u32,
        /// Number of minutes
        #[arg(short = 'M', default_value_t = 0, hide_default_value = true)]
        minutes: u32,
    },
    #[clap(about = "Sessions ranging between a specific time, and now", long_about = SINCE_ABOUT)]
    Since {
        /// Start of the range
        #[arg(value_parser = parse_abs)]
        start: Option<Absolute>,
    },
    #[clap(about = "Sessions ranging between two specific times", long_about = RANGE_ABOUT)]
    Range {
        /// Start of the range
        #[arg(short, value_parser = parse_abs)]
        start: Option<Absolute>,
        /// End of the range
        #[arg(short, value_parser = parse_abs)]
        end: Option<Absolute>,
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
    use Command::*;
    let cli = Cli::try_parse()?;

    match cli.command {
        New { name } => commands::create(name),
        Set { name } => commands::set(name),
        Delete { name } => commands::delete(name),
        Rename { from, to } => commands::rename(from, to),
        Current => commands::current(),
        All => commands::all(),
        Start => commands::start(),
        End { notes } => commands::end(notes),
        Cancel => commands::cancel(),
        Ongoing => commands::ongoing(),
        Add { start, end, notes } => commands::add(start, end, notes),
        Edit {
            position,
            start,
            end,
            notes,
        } => commands::edit(position, start, end, notes),
        Remove { position } => commands::remove(position),
        List { range_command } => {
            let (start, end) = get_bounds(range_command);
            commands::list(start, end)
        }
        Stats { range_command } => {
            let (start, end) = get_bounds(range_command);
            commands::stats(start, end)
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
            RangeCommand::Since { start } => (to_bound(start), Bound::Now),
            RangeCommand::Range { start, end } => (to_bound(start), to_bound(end)),
            RangeCommand::On { date } => (
                Bound::Absolute(Absolute::Date(date)),
                Bound::Absolute(Absolute::Date(date)),
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
        return Ok(Position::Last);
    } else if let Ok(i) = s.parse() {
        if i > 0 {
            return Ok(Position::Index(i));
        }
    }
    Err("index must be either [last] or a positive integer".to_string())
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%d/%m/%y")
        .map_err(|_| "date must be in the form [dd/mm/yy]".to_string())
}

fn parse_abs(s: &str) -> Result<Absolute, String> {
    if let Ok(date_time) = NaiveDateTime::parse_from_str(s, "%d/%m/%y-%R") {
        return Ok(Absolute::DateTime(date_time));
    } else if let Ok(date) = NaiveDate::parse_from_str(s, "%d/%m/%y") {
        return Ok(Absolute::Date(date));
    } else if let Ok(time) = NaiveTime::parse_from_str(s, "%R") {
        return Ok(Absolute::Time(time));
    }
    Err("must be in the form [dd/mm/yy] or [HH:MM] or [dd/mm/yy-HH:MM]".to_string())
}

fn to_bound(abs: Option<Absolute>) -> Bound {
    if let Some(abs) = abs {
        Bound::Absolute(abs)
    } else {
        Bound::None
    }
}

const ADD_ABOUT: &str = "Add a new session
    
<START>: [dd/mm/yy-HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 on dd/mm/yy
         [HH:MM]          - HH:MM on today's date

<END>:   [dd/mm/yy-HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 the day after dd/mm/yy
         [HH:MM]          - HH:MM on <START>'s date";

const EDIT_ABOUT: &str = "Edit a session

<POSITION>: [index]          - index of the session, as shown in track list
            \"last\"           - last recorded session
    
<START>:    [dd/mm/yy-HH:MM] - HH:MM on dd/mm/yy
            [dd/mm/yy]       - 00:00 on dd/mm/yy
            [HH:MM]          - HH:MM on today's date
            omitted          - leave unchanged

<END>:      [dd/mm/yy-HH:MM] - HH:MM on dd/mm/yy
            [dd/mm/yy]       - 00:00 the day after dd/mm/yy
            [HH:MM]          - HH:MM on <START>'s date
            omitted          - leave unchanged

<NOTES>:    [string]         - string
            whitespace       - remove notes
            omitted          - leave unchanged";

const REMOVE_ABOUT: &str = "Remove a session

<POSITION>: [index]          - index of the session, as shown in track list
            \"last\"           - last recorded session";

const LIST_ABOUT: &str = "Display full session history, or sessions in a specific time range

Omit [COMMAND] for full session history";

const STATS_ABOUT: &str =
    "Display full session statistics, or sessions statistics in a specific time range

Omit [COMMAND] for full session statistics";

const PAST_ABOUT: &str = "Sessions ranging between a specific amount of time in the past, and now

Omit all arguments to start from the first recorded session";

const SINCE_ABOUT: &str = "Sessions ranging between a specific time, and now

<START>: [dd/mm/yy-HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 on dd/mm/yy
         [HH:MM]          - HH:MM on today's date
         omitted          - start of first recorded session";

const RANGE_ABOUT: &str = "Sessions ranging between two specific times

<START>: [dd/mm/yy-HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 on dd/mm/yy
         [HH:MM]          - HH:MM on today's date
         omitted          - start of first recorded session
         
<END>:   [dd/mm/yy-HH:MM] - HH:MM on dd/mm/yy
         [dd/mm/yy]       - 00:00 the day after dd/mm/yy
         [HH:MM]          - HH:MM on <START>'s date
         omitted          - end of last recorded session";
