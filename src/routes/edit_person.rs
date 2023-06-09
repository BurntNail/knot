use crate::{
    auth::{get_auth_object, Auth, PermissionsRole},
    error::KnotError,
    liquid_utils::{compile, EnvFormatter},
    routes::DbPerson,
    state::KnotState,
};
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect},
    Form,
};
use serde::{Deserialize, Serialize};
use crate::routes::{rewards::Reward, add_person::NoIDPerson};

#[instrument(level = "debug", skip(auth, state))]
pub async fn get_edit_person(
    auth: Auth,
    Path(id): Path<i32>,
    State(state): State<KnotState>,
) -> Result<impl IntoResponse, KnotError> {
    #[derive(Serialize)]
    pub struct SmolPerson {
        pub id: i32,
        pub permissions: PermissionsRole,
        pub first_name: String,
        pub surname: String,
        pub username: String,
        pub password_is_set: bool,
        pub form: String,
    }

    debug!("Getting relevant person");

    let person = sqlx::query_as!(
        DbPerson,
        r#"
SELECT id, first_name, surname, username, form, hashed_password, permissions as "permissions: _", was_first_entry
FROM people WHERE id = $1
        "#,
        id
    )
    .fetch_one(&mut state.get_connection().await?)
    .await?;
    let person = SmolPerson {
        id: person.id,
        permissions: person.permissions,
        first_name: person.first_name,
        surname: person.surname,
        username: person.username,
        form: person.form,
        password_is_set: person.hashed_password.is_some(),
    };

    debug!("Getting events supervised");

    #[derive(Serialize)]
    struct Event {
        name: String,
        date: String,
        id: i32,
        verified: bool,
    }

    let events_supervised = sqlx::query!(
        r#"
SELECT date, event_name, id FROM events e 
INNER JOIN prefect_events pe
ON pe.event_id = e.id AND pe.prefect_id = $1
        "#,
        person.id
    )
    .fetch_all(&mut state.get_connection().await?)
    .await?
    .into_iter()
    .map(|r| Event {
        name: r.event_name,
        date: r.date.to_env_string(),
        id: r.id,
        verified: true,
    })
    .collect::<Vec<_>>();

    debug!("Getting events participated");

    let events_participated = sqlx::query!(
        r#"
SELECT date, event_name, id, is_verified FROM events e 
INNER JOIN participant_events pe
ON pe.event_id = e.id AND pe.participant_id = $1 AND pe.is_verified = true
        "#,
        person.id
    )
    .fetch_all(&mut state.get_connection().await?)
    .await?
    .into_iter()
    .map(|r| Event {
        name: r.event_name,
        date: r.date.to_env_string(),
        verified: r.is_verified,
        id: r.id,
    })
    .collect::<Vec<_>>();

    let rewards = sqlx::query_as!(Reward, "select name, first_entry_pts, second_entry_pts, id FROM rewards_received rr inner join rewards r on r.id = rr.reward_id and rr.person_id = $1", person.id).fetch_all(&mut state.get_connection().await?).await?;


    debug!("Compiling");

    compile("www/edit_person.liquid", liquid::object!({ "person": person, "supervised": events_supervised, "participated": events_participated, "rewards": rewards,  "auth": get_auth_object(auth) })).await
}

#[instrument(level = "debug", skip(state, first_name, surname))]
pub async fn post_edit_person(
    Path(id): Path<i32>,
    State(state): State<KnotState>,
    Form(NoIDPerson {
        first_name,
        surname,
        form,
        username,
        permissions,
    }): Form<NoIDPerson>,
) -> Result<impl IntoResponse, KnotError> {
    debug!("Editing person");
    sqlx::query!(
        r#"
UPDATE public.people
SET permissions=$6, first_name=$2, surname=$3, form=$4, username=$5
WHERE id=$1
        "#,
        id,
        first_name,
        surname,
        form,
        username,
        permissions as _
    )
    .execute(&mut state.get_connection().await?)
    .await?;

    Ok(Redirect::to(&format!("/edit_person/{id}")))
}

#[derive(Deserialize)]
pub struct PasswordReset {
    id: i32,
}

#[instrument(level = "debug", skip(auth, state))]
pub async fn post_reset_password(
    mut auth: Auth,
    State(state): State<KnotState>,
    Form(PasswordReset { id }): Form<PasswordReset>,
) -> Result<impl IntoResponse, KnotError> {
    debug!("Logging out.");

    if auth
        .current_user
        .clone()
        .expect("user logged in to reset password")
        .id
        == id
    {
        auth.logout().await;
    }

    debug!("Sending password reset");
    state.reset_password(id).await?;
    Ok(Redirect::to("/show_all"))
}
