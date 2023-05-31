use super::public::serve_static_file;
use crate::{error::KnotError};
use axum::{extract::State, response::IntoResponse};
use rust_xlsxwriter::{Color, Format, FormatAlign, Workbook};
use sqlx::{Pool, Postgres};
use std::{collections::HashMap, sync::Arc};
use tokio::task;

pub async fn get_spreadsheet(
    State(pool): State<Arc<Pool<Postgres>>>,
) -> Result<impl IntoResponse, KnotError> {
    let mut people = sqlx::query!(
        r#"
SELECT id, first_name, surname, form  FROM people"#
    )
    .fetch_all(pool.as_ref())
    .await?;
    people.sort_by_key(|x| x.surname.clone());
    people.sort_by_key(|x| x.form.clone());

    let mut events = sqlx::query!(
        r#"
SELECT * FROM events"#
    )
    .fetch_all(pool.as_ref())
    .await?;
    events.sort_by_key(|r| r.date);

    let mut participant_relationships = HashMap::new();
    sqlx::query!("SELECT participant_id, event_id FROM participant_events")
        .fetch_all(pool.as_ref())
        .await?
        .into_iter()
        .for_each(|x| {
            participant_relationships
                .entry(x.participant_id)
                .or_insert(vec![])
                .push(x.event_id);
        });

    task::spawn_blocking(move || -> Result<(), KnotError> {
        let mut workbook = Workbook::new();

        let title_fmt = Format::new()
            .set_background_color(Color::Gray)
            .set_align(FormatAlign::Center)
            .set_align(FormatAlign::VerticalCenter)
            .set_bold();
        let event_fmt = Format::new()
            .set_background_color(Color::Yellow)
            .set_rotation(90);
        let person_fmt = Format::new().set_background_color(Color::Green);

        let sheet = workbook.add_worksheet();

        sheet.write_with_format(2, 0, "Name", &title_fmt)?;
        sheet.write_with_format(2, 1, "Form", &title_fmt)?;
        sheet.write_with_format(2, 2, "Total Events", &title_fmt)?;

        sheet.merge_range(0, 0, 0, 2, "Event Name", &title_fmt)?;
        sheet.merge_range(1, 0, 1, 2, "Event Date", &title_fmt)?;

        let mut events_to_check = vec![];

        for (col, event) in events.into_iter().enumerate() {
            let col = col as u16 + 3;
            sheet.write_with_format(0, col, &event.event_name, &event_fmt)?;
            sheet.write_with_format(
                1,
                col,
                &event.date.format("%d/%m/%Y").to_string(),
                &event_fmt,
            )?;
            events_to_check.push((col, event.id));
        }

        for (
            row,
            rec
        ) in people
            .into_iter()
            .enumerate()
            .map(|(row, db)| (row + 3, db))
        {
            let row = row as u32;

            let pr = participant_relationships.remove(&rec.id).unwrap_or_default();
            sheet.write_with_format(row, 0, format!("{} {}", rec.first_name, rec.surname), &person_fmt)?;
            sheet.write_with_format(row, 1, &rec.form, &person_fmt)?;
            sheet.write_with_format(row, 2, &pr.len().to_string(), &person_fmt)?;

            for (col, event_id) in &events_to_check {
                if pr.contains(event_id) {
                    sheet.write(row, *col, 1.0)?;
                }
            }
        }

        workbook.save("student_spreadsheet.xlsx")?;

        Ok(())
    })
    .await??;

    serve_static_file("student_spreadsheet.xlsx").await
}
