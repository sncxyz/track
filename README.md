# track

A CLI for tracking time spent on different activities

## Installation

`cargo install --git https://github.com/sncxyz/track`

## Usage

`track <COMMAND>`

| Command | Description                                                                         |
| ------- | ----------------------------------------------------------------------------------- |
| new     | Create a new activity to track                                                      |
| set     | Set the active activity that other commands should act on                           |  
| rename  | Rename an activity                                                                  |  
| delete  | Delete an activity                                                                  |
| current | Display the name of the active activity                                             |
| all     | Display the names of all tracked activities                                         |
| start   | Start tracking a session                                                            |
| end     | End tracking of the ongoing session                                                 |
| cancel  | Cancel tracking of the ongoing session                                              |
| ongoing | Display details of the ongoing session                                              |
| add     | Add a new session                                                                   |
| past    | Add a new session that ends at the current time                                     |
| edit    | Edit a session                                                                      |
| remove  | Remove a session                                                                    |
| view    | Display full session history, or sessions in a specific time range                  |
| stats   | Display full session statistics, or session statistics in a specific time range     |