use crate::komga::KomgaEntry;
use anyhow::{bail, Result};
use inquire::{Select, Text};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use std::fmt::{Display, Formatter};

const INFO_QUERY: &str = "\
query ($id: Int) {
  Media (id: $id, type: MANGA) {
    title {
      romaji
      english
    }
    coverImage {
      extraLarge
      large
      medium
    }
    description
    status
    genres
    staff {
      edges {
        node {
          name {
            full
          }
        }
        role
      }
    }
  }
}";

const SEARCH_QUERY: &str = "\
query ($perPage: Int, $search: String) {
  Page (page: 1, perPage: $perPage) {
    media (search: $search, type: MANGA) {
      id
      title {
        english
        romaji
      }
    }
  }
}";

#[derive(Deserialize)]
struct SearchResponse {
    data: SearchData,
}

#[derive(Deserialize)]
struct SearchData {
    #[serde(rename = "Page")]
    page: SearchPage,
}

#[derive(Deserialize)]
struct SearchPage {
    media: Vec<SearchMedia>,
}

#[derive(Deserialize)]
struct SearchMedia {
    id: u32,
    title: AnilistTitle,
}

#[derive(Deserialize)]
struct AnilistResponse {
    data: AnilistData,
}

#[derive(Deserialize)]
struct AnilistData {
    #[serde(rename = "Media")]
    media: AnilistMedia,
}

#[derive(Deserialize)]
struct AnilistMedia {
    title: AnilistTitle,
    #[serde(rename = "coverImage")]
    cover_image: AnilistCover,
    description: Option<String>,
    status: AnilistStatus,
    genres: Vec<String>,
    staff: AnilistStaff,
}

#[derive(Deserialize)]
struct AnilistTitle {
    romaji: Option<String>,
    english: Option<String>,
}

#[derive(Deserialize)]
struct AnilistCover {
    #[serde(rename = "extraLarge")]
    extra_large: Option<String>,
    large: Option<String>,
    medium: Option<String>,
}

#[derive(Deserialize)]
struct AnilistStaff {
    edges: Vec<AnilistStaffEdge>,
}

#[derive(Deserialize)]
struct AnilistStaffEdge {
    node: AnilistStaffNode,
    role: String,
}

#[derive(Deserialize)]
struct AnilistStaffNode {
    name: AnilistStaffNodeName,
}

#[derive(Deserialize)]
struct AnilistStaffNodeName {
    full: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AnilistStatus {
    #[serde(rename = "FINISHED")]
    Finished,
    #[serde(rename = "RELEASING")]
    Releasing,
    #[serde(rename = "NOT_YET_RELEASED")]
    NotYetReleased,
    #[serde(rename = "CANCELLED")]
    Cancelled,
    #[serde(rename = "HIATUS")]
    Hiatus,
}

#[derive(Debug)]
pub struct MangaInfo {
    pub title: String,
    pub cover: Option<String>,
    pub description: Option<String>,
    pub status: AnilistStatus,
    pub genres: Vec<String>,
    pub artist: Option<String>,
    pub author: Option<String>,
}

#[derive(Clone)]
struct StaffOption {
    name: String,
    role: String,
}

impl Display for StaffOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.name, self.role)
    }
}

impl Display for SearchMedia {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let title = match &self.title {
            AnilistTitle {
                english: Some(eng),
                romaji: Some(rom),
            } => {
                format!("{} / {}", eng, rom)
            }
            AnilistTitle {
                english: Some(eng),
                romaji: None,
            } => eng.into(),
            AnilistTitle {
                english: None,
                romaji: Some(rom),
            } => rom.into(),
            _ => {
                return Err(fmt::Error);
            }
        };
        write!(f, "{}", title)
    }
}

pub fn get_anilist_data(id: u32) -> Result<MangaInfo> {
    let anilist_client = reqwest::blocking::Client::new();
    let json = json!(
        {"query": INFO_QUERY, "variables": {"id": id}}
    );

    let resp: AnilistResponse = anilist_client
        .post("https://graphql.anilist.co/")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(json.to_string())
        .send()?
        .json()?;

    let data = resp.data.media;
    let title = match data.title {
        AnilistTitle {
            english: Some(eng),
            romaji: Some(rom),
        } => {
            if eng == rom {
                eng
            } else {
                let options = vec![eng, rom];
                Select::new("Select title", options).prompt()?
            }
        }
        AnilistTitle {
            english: Some(eng),
            romaji: None,
        } => eng,
        AnilistTitle {
            english: None,
            romaji: Some(rom),
        } => rom,
        _ => bail!("No title available"),
    };

    let staff: Vec<StaffOption> = data
        .staff
        .edges
        .iter()
        .map(|s| StaffOption {
            name: s.node.name.full.clone(),
            role: s.role.clone(),
        })
        .collect();
    let author = get_staff_name(staff.clone(), "Select author:")?;
    let artist = get_staff_name(staff, "Select artist:")?;

    let tag_regex = Regex::new(r"</?\w+>").unwrap();
    let summary = data
        .description
        .map(|s| tag_regex.replace_all(s.as_str(), "").into_owned());

    Ok(MangaInfo {
        title,
        cover: data
            .cover_image
            .extra_large
            .or(data.cover_image.large.or(data.cover_image.medium)),
        description: summary,
        status: data.status,
        genres: data.genres,
        artist,
        author,
    })
}

fn get_staff_name(options: Vec<StaffOption>, title: &str) -> Result<Option<String>> {
    let ans = Select::new(title, options)
        .with_help_message("↑↓ to move, enter to select, type to filter, [Esc] to select none")
        .prompt_skippable()?
        .map(|a| a.name);
    Ok(ans)
}

pub fn search_manga(entry: &KomgaEntry) -> Result<u32> {
    let search_query = Text::new("AniList search query:")
        .with_initial_value(&entry.name)
        .prompt()?;

    let anilist_client = reqwest::blocking::Client::new();
    let json = json!(
        {"query": SEARCH_QUERY, "variables": {"search": search_query, "perPage": 20}}
    );

    let resp: SearchResponse = anilist_client
        .post("https://graphql.anilist.co/")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .body(json.to_string())
        .send()?
        .json()?;

    let id = Select::new("Select entry:", resp.data.page.media)
        .prompt()?
        .id;

    Ok(id)
}
