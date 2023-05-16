use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::Form;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

use crate::{error::KnotError, liquid_utils::compile};

use super::Person;

pub const LOCATION: &str = "/kingsleyisbest123/remove_stuff";

#[derive(Deserialize)]
pub struct SmolDbEvent {
    pub id: i32,
    pub event_name: String,
    pub date: NaiveDateTime,
}
#[derive(Serialize)]
pub struct SmolFormattedDbEvent {
    pub id: i32,
    pub event_name: String,
    pub date: String,
}

impl From<SmolDbEvent> for SmolFormattedDbEvent {
    fn from(
        SmolDbEvent {
            id,
            event_name,
            date,
        }: SmolDbEvent,
    ) -> Self {
        Self {
            id,
            event_name,
            date: date.format("%d/%m/%Y @ %H:%M").to_string(),
        }
    }
}

pub async fn get_remove_stuff(
    State(pool): State<Arc<Pool<Postgres>>>,
) -> Result<impl IntoResponse, KnotError> {
    let mut conn = pool.acquire().await?;

    let people: Vec<Person> = sqlx::query_as!(
        Person,
        r#"
SELECT *
FROM people
        "#
    )
    .fetch_all(&mut conn)
    .await?;

    let events: Vec<SmolFormattedDbEvent> = sqlx::query_as!(
        SmolDbEvent,
        r#"
SELECT id, event_name, date
FROM events
        "#
    )
    .fetch_all(&mut conn)
    .await?
    .into_iter()
    .map(SmolFormattedDbEvent::from)
    .collect();

    let globals = liquid::object!({
        "people": people,
        "events": events
    });

    compile("www/remove_stuff.liquid", globals).await
}

#[derive(Deserialize)]
pub struct RemovePerson {
    pub person_id: Vec<i32>,
}

#[derive(Deserialize)]
pub struct RemoveEvent {
    pub event_id: Vec<i32>,
}

pub async fn post_remove_person(
    State(pool): State<Arc<Pool<Postgres>>>,
    Form(RemovePerson { person_id }): Form<RemovePerson>,
) -> Result<impl IntoResponse, KnotError> {
    let mut conn = pool.acquire().await?;

    for person_id in person_id {
        sqlx::query!(
            r#"
    DELETE FROM public.people
    WHERE id=$1
            "#,
            person_id
        )
        .execute(&mut conn)
        .await?;
    }

    Ok(Redirect::to(LOCATION))
}
pub async fn post_remove_event(
    State(pool): State<Arc<Pool<Postgres>>>,
    Form(RemoveEvent { event_id }): Form<RemoveEvent>,
) -> Result<impl IntoResponse, KnotError> {
    let mut conn = pool.acquire().await?;

    for event_id in event_id {
        sqlx::query!(
            r#"
    DELETE FROM public.events
    WHERE id=$1
            "#,
            event_id
        )
        .execute(&mut conn)
        .await?;
    }

    Ok(Redirect::to(LOCATION))
}
