pub mod add_event;
pub mod add_people_to_event;
pub mod add_person;
pub mod calendar;
pub mod icon;
pub mod index;
pub mod remove_stuff;
pub mod update_event_and_person;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::error::KnotError;

#[derive(Deserialize, Serialize)]
struct Person {
    pub person_name: String,
    pub is_prefect: bool,
    pub id: i32,
}

#[derive(Deserialize)]
pub struct DbEvent {
    pub id: i32,
    pub event_name: String,
    pub date: NaiveDateTime,
    pub location: String,
    pub teacher: String,
    pub other_info: Option<String>,
}

///Struct to hold the event that comes back from the [`add_event`] form
///
/// NB: when going into a [`DbEvent`], the ID will be -1,
#[derive(Debug, Deserialize)]
pub struct FormEvent {
    pub name: String,
    pub date: String,
    pub location: String,
    pub teacher: String,
    pub info: String,
}

impl TryFrom<FormEvent> for DbEvent {
    type Error = KnotError;

    ///Get a [`DbEvent`] from a [`FormEvent`], can fail if we can't parse the date.
    ///
    /// NB: Event ID is always -1 as `try_from` cannot get a DB connection
    fn try_from(
        FormEvent {
            name,
            date,
            location,
            teacher,
            info,
        }: FormEvent,
    ) -> Result<Self, Self::Error> {
        let date = NaiveDateTime::parse_from_str(&date, "%Y-%m-%dT%H:%M")?;

        Ok(Self {
            id: -1, //no ID for events to be added
            event_name: name,
            date,
            location,
            teacher,
            other_info: Some(info),
        })
    }
}
