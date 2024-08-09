use crate::anilist::{AnilistStatus, MangaInfo};
use crate::config::KomgaConfig;
use anyhow::Result;
use inquire::Select;
use reqwest::blocking::{Client, Response};
use serde::Deserialize;
use serde_json::{json, Value};
use std::fmt::{Display, Formatter};

const API_SLUG: &str = "api/v1";

fn send_get_request(
    cfg: &KomgaConfig,
    url_slug: &str,
    query: &[(&str, &str)],
) -> reqwest::Result<Response> {
    let client = Client::new();
    client
        .get(format!("{}/{}/{}", cfg.url, API_SLUG, url_slug))
        .query(query)
        .basic_auth(&cfg.email, Some(&cfg.password).filter(|s| !s.is_empty()))
        .send()
}

fn send_patch_request(
    cfg: &KomgaConfig,
    url_slug: &str,
    query: &[(&str, &str)],
    body: Value,
) -> reqwest::Result<Response> {
    let client = Client::new();
    client
        .patch(format!("{}/{}/{}", cfg.url, API_SLUG, url_slug))
        .query(query)
        .json(&body)
        .basic_auth(&cfg.email, Some(&cfg.password).filter(|s| !s.is_empty()))
        .send()
}

#[derive(Debug, Deserialize)]
struct SeriesResponse {
    content: Vec<KomgaEntry>,
}

#[derive(Debug, Deserialize)]
pub struct KomgaEntry {
    pub id: String,
    pub name: String,
}

impl Display for KomgaEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub fn get_library(cfg: &KomgaConfig) -> Result<String> {
    let libraries: Vec<KomgaEntry> = send_get_request(cfg, "libraries", &[])?.json()?;

    let library_id = if libraries.len() == 1 {
        libraries[0].id.clone()
    } else {
        Select::new("Select library:", libraries)
            .with_help_message("↑↓ to move, enter to select, type to filter, [Esc] to select none")
            .prompt()?
            .id
    };

    Ok(library_id)
}

pub fn get_entry(cfg: &KomgaConfig, library_id: &str) -> Result<KomgaEntry> {
    let entries: SeriesResponse = send_get_request(
        cfg,
        "series",
        &[
            ("sort", "metadata.titleSort,asc"),
            ("library_id", library_id),
        ],
    )?
    .json()?;

    let selected = Select::new("Select entry:", entries.content).prompt()?;

    Ok(selected)
}

pub fn update_info(cfg: &KomgaConfig, info: &MangaInfo, series_id: &str) -> Result<()> {
    let json = json!({
        "genres": info.genres,
        "summary": info.description,
        "title": info.title,
        "titleSort": info.title,
        "status": get_komga_status(&info.status),
    });

    send_patch_request(cfg, &format!("series/{}/metadata", series_id), &[], json)?;

    if info.author.is_some() || info.artist.is_some() {
        let book_response: SeriesResponse = send_get_request(
            cfg,
            &format!("series/{}/books", series_id),
            &[
                ("sort", "metadata.numberSort,asc"),
                ("size", "1"),
                ("page", "0"),
            ],
        )?
        .json()?;

        let book_id = &book_response.content[0].id;

        let mut book_info: Vec<Value> = Vec::new();
        if let Some(author) = &info.author {
            book_info.push(json!({
                "name": author,
                "role": "writer"
            }));
        }
        if let Some(artist) = &info.artist {
            book_info.push(json!({
                "name": artist,
                "role": "penciller"
            }));
        }

        let book_body = json!({
            "authors": book_info
        });

        send_patch_request(cfg, &format!("books/{}/metadata", book_id), &[], book_body)?;
    }

    Ok(())
}

fn get_komga_status(status: &AnilistStatus) -> String {
    match status {
        AnilistStatus::Finished => "ENDED".into(),
        AnilistStatus::Releasing => "ONGOING".into(),
        AnilistStatus::NotYetReleased => "ONGOING".into(),
        AnilistStatus::Cancelled => "ABANDONED".into(),
        AnilistStatus::Hiatus => "HIATUS".into(),
    }
}
