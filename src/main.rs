use serde::{Deserialize, Serialize};

use anyhow::{ensure, Context, Result};

use itertools::Itertools;

use clap::clap_app;

use colored::Colorize;

use std::fs;
use std::env;

#[derive(Debug, Deserialize, Serialize)]
struct Repo {
    name: String,
    #[serde(rename = "html_url")]
    url: String,
    description: String,
    #[serde(rename = "stargazers_count")]
    star_count: u64,
}

fn main() {
    let args = clap_app!(Twitch_cli =>
                         (version: "0.1.0")
                         (author: "Constantin Loew")
                         (@arg USER: -u --user +takes_value "Which user to get the starred repos from")
                         (@arg CLEAR: -c --clear-cache "Clears cache")
                         (@arg JSON: -j --json +takes_value "")
                         (@arg TOML: -t --toml +takes_value "")
    )
    .get_matches();

    if args.is_present("CLEAR") {
        clear_cache();
    }

    match args.value_of("USER") {
        Some(user) => {
            match get_starred_repos_for_user(&user) {
                Ok(repos) => {
                    // if user wants file output silence terminal
                    if args.value_of("JSON").is_some() || args.value_of("TOML").is_some() {
                        // write toml to TOML
                        if let Some(toml_file) = args.value_of("TOML") {
                            match toml::to_string(&repos) {
                                Ok(toml_string) => {
                                    if let Err(err) = fs::write(toml_file, toml_string) {
                                        println!("Writing to {} failed with {:?}", toml_file, err);
                                    }
                                }
                                Err(err) => println!("Failed serializing toml with {:?}", err),
                            }

                        }
                        // write json to JSON
                        if let Some(json_file) = args.value_of("JSON") {
                            match serde_json::to_string(&repos) {
                                Ok(json_string) => {
                                    if let Err(err) = fs::write(json_file, json_string) {
                                        println!("Writing to {} failed with {:?}", json_file, err);
                                    }
                                }
                                Err(err) => println!("Failed serializing json with {:?}", err),
                            }

                        }
                    } else { // else print repos to terminal
                        list_repos(&repos);
                    }

                },
                Err(err) => println!("ERROR: {:?}", err),
            }
        }
        None => println!("No user was specified"),
    }
}

fn get_starred_repos_for_user(user: &str) -> Result<Vec<Repo>> {
    if let Some(cached_response) = get_cache(user) {
        let repos: Vec<Repo> = serde_json::from_str(&cached_response)?;
        return Ok(repos);
    }

    let client = reqwest::blocking::Client::new();
    let url = format!("https://api.github.com/users/{}/starred?per_page=10", user);

    let access_token =
        env::var("GITHUB_ACCESS").context("Could not get access token, is TWITCH_ACCESS set?")?;

    let req = client
        .get(&url)
        .header("User-Agent", "starred-repos")
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header(reqwest::header::AUTHORIZATION, &format!("Bearer {}", access_token));

    let res = req.send().context("Could not connect to github api")?;

    ensure!(
        res.status().is_success(),
        "{} returned error with status {}",
        url,
        res.status()
    );

    let response_text = res.text()?;
    write_cache(user, &response_text);

    let repos: Vec<Repo> = serde_json::from_str(&response_text)?;
    Ok(repos)
}

fn write_cache(user: &str, response: &str) {
    if fs::read_dir("cache").is_err() {
        if let Err(err) = fs::create_dir("cache") {
            println!("Error creating cache: {:?}", err);
        }
    }
    match fs::write(format!("cache/{}", user), response) {
        Ok(_) => (),
        Err(err) => println!("Error writing to cache: {:?}", err),
    }
}

fn get_cache(user: &str) -> Option<String> {
    match fs::read_to_string(format!("cache/{}", user)) {
        Ok(cached_response) => Some(cached_response),
        Err(_) => None
    }
}

fn clear_cache() {
    if let Err(err) = fs::remove_dir_all("cache") {
        println!("Failed clearing cache with {:?}", err);
    }
}

fn list_repos(repos: &[Repo]) {
    for repo in repos
        .into_iter()
        .sorted_by(|a, b| Ord::cmp(&b.star_count, &a.star_count))
    {
        println!(
            "{}\n\t{}{}\n\t{}{}\n\t{}{}",
            repo.name.bold(), format!("Stars:       ").yellow(), repo.star_count, format!("Description: ").blue(), repo.description, format!("URL:         ").green(), repo.url
        );
    }


}
