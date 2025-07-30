use anyhow::{Context, Result};
use duckduckgo_rs::{search_duckduckgo, SearchResult};
use reqwest::Client;
use scraper::{Html, Selector};

/// Searches the web using DuckDuckGo, scrapes the top results, and returns the combined text content.
pub async fn search_and_scrape(http_client: &Client, query: &str) -> Result<String> {
    // 1. Search DuckDuckGo
    let search_results: Vec<SearchResult> = search_duckduckgo(http_client, query)
        .await
        .context("Failed to get search results from DuckDuckGo")?;

    let mut scraped_content = Vec::new();

    // 2. Scrape the top 2 results
    for result in search_results.iter().take(2) {
        scraped_content.push(result.description.clone());

        if let Ok(response) = http_client.get(&result.url).send().await {
            if let Ok(html_content) = response.text().await {
                let document = Html::parse_document(&html_content);
                if let Ok(selector) = Selector::parse("p, h1, h2, h3, code") {
                    let text = document
                        .select(&selector)
                        .map(|el| el.text().collect::<Vec<_>>().join(" "))
                        .collect::<Vec<_>>()
                        .join("\n");
                    scraped_content.push(text);
                }
            }
        }
    }

    Ok(scraped_content.join("\n---\n"))
}