# track

A CLI for tracking time spent on different activities

## Installation

`cargo install --git https://github.com/scjqt/track`

## Usage

`track <COMMAND>`

| Command  | Description                                                                         |
| -------- | ----------------------------------------------------------------------------------- |
|  new     | Create a new activity to track                                                      |
|  set     | Set the activity that other commands should act on                                  |  
|  delete  | Delete an activity                                                                  |
|  current | Display the name of the current activity                                            |
|  all     | Display the names of all tracked activities                                         |
|  start   | Start tracking a session                                                            |
|  end     | End tracking of the ongoing session                                                 |
|  cancel  | Cancel tracking of the ongoing session                                              |
|  ongoing | Display details of the ongoing session                                              |
|  add     | Add a new session                                                                   |
|  edit    | Edit a session                                                                      |
|  remove  | Remove a session                                                                    |
|  list    | Display full session history, or sessions in a specific time range                  |
|  stats   | Display full session statistics, or session statistics in a specific time range     |
|  help    | Print this message or the help of the given subcommand(s)                           |