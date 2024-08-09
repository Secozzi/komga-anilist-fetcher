mod anilist;
mod config;
mod komga;

use crate::anilist::{get_anilist_data, search_manga};
use crate::config::{get_config, KomgaConfig};
use crate::komga::{get_entry, get_library, update_info};
use anyhow::Result;

fn main() -> Result<()> {
    let cfg: KomgaConfig = get_config()?;

    let selected_library = get_library(&cfg)?;
    let selected_entry = get_entry(&cfg, &selected_library)?;
    let anilist_entry = search_manga(&selected_entry)?;
    let anilist_info = get_anilist_data(anilist_entry)?;

    update_info(&cfg, &anilist_info, &selected_entry.id)?;

    Ok(())
}
