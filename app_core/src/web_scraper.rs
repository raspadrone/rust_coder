// in app_core/src/web_scraper.rs

use anyhow::{Context, Result};
use reqwest::Url;
use scraper::{Html, Selector};
use std::collections::{HashSet, VecDeque};

/// Scrapes a website starting from a given URL, collecting all text content.
pub async fn scrape_website(start_url: &str) -> Result<String> {
    let start_url = Url::parse(start_url).context("Failed to parse start URL")?;
    let domain = start_url.domain().context("URL has no domain")?.to_string();

    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut all_text = String::new();

    queue.push_back(start_url.clone());
    visited.insert(start_url.to_string());

    println!("Starting scrape of domain: {}", domain);

    while let Some(current_url) = queue.pop_front() {
        println!("Scraping: {}", current_url);

        // Fetch the page content
        let response = reqwest::get(current_url.clone()).await?;
        if !response.status().is_success() {
            println!("Warning: Failed to fetch {}: {}", current_url, response.status());
            continue;
        }

        let body = response.text().await?;
        let document = Html::parse_document(&body);

        // 1. Extract and append the text content from the current page
        let text = extract_text_from_html(&document);
        all_text.push_str(&text);
        all_text.push_str("\n\n"); // Add separation between pages

        // 2. Find and queue new links on the same domain
        let links = find_links_on_page(&document, &current_url, &domain);
        for link in links {
            if visited.insert(link.to_string()) {
                queue.push_back(link);
            }
        }
    }

    println!("Scrape complete. Total characters found: {}", all_text.len());
    Ok(all_text)
}

/// Extracts all visible text from an HTML document.
fn extract_text_from_html(document: &Html) -> String {
    // We select the `body` tag to avoid scraping text from `<head>`, `<script>`, etc.
    let body_selector = Selector::parse("body").unwrap();
    if let Some(body_node) = document.select(&body_selector).next() {
        body_node.text().collect::<String>()
    } else {
        String::new()
    }
}

/// Finds all valid, same-domain links on a page.
fn find_links_on_page(document: &Html, base_url: &Url, domain: &str) -> HashSet<Url> {
    let link_selector = Selector::parse("a[href]").unwrap();
    let mut valid_links = HashSet::new();

    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            // Use the base_url to resolve relative links (e.g., "/about-us")
            if let Ok(mut full_url) = base_url.join(href) {
                // Clean the URL by removing fragments (#) and query params (?)
                full_url.set_fragment(None);
                full_url.set_query(None);

                // Only include HTTP/HTTPS links on the same domain
                if (full_url.scheme() == "http" || full_url.scheme() == "https")
                    && full_url.domain() == Some(domain)
                {
                    valid_links.insert(full_url);
                }
            }
        }
    }
    valid_links
}