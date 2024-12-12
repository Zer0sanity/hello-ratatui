use std::{fmt, string::ToString};

use serde::{
    Deserialize, Serialize,
    de::{self, Deserializer, Visitor},
};
use strum::Display;

//// ANCHOR: action_enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Display, Deserialize)]
pub enum Action {
    Tick,
    Render,
    Resize(u16, u16),
    Suspend,
    Resume,
    Quit,
    Refresh,
    Error(String),
    Help,
    ToggleShowHelp,
    IncrementSingle,
    DecrementSingle,
    ScheduleIncrement,
    ScheduleDecrement,
    Increment(usize),
    Decrement(usize),
    CompleteInput(String),
    EnterNormal,
    EnterInsert,
    EnterProcessing,
    ExitProcessing,
    Update,
}
