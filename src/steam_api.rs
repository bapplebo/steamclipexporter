use reqwest::blocking::Client;

use crate::AppDetails;

pub fn get_app_details(steam_id: u64) -> Result<AppDetails, reqwest::Error> {
    println!("Fetching app details from Steam API...");

    let client = Client::new();
    let url = format!(
        "https://store.steampowered.com/api/appdetails?appids={}",
        steam_id
    );
    let response = client.get(&url).send()?;
    response.error_for_status_ref()?;

    let app_details = response.json()?;
    Ok(app_details)
}
