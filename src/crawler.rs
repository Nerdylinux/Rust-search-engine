use scraper::{Html, Selector};
use std::collections::HashSet;
use url::Url;

pub struct Page {
    pub url: String,
    pub title: String,
    pub body: String,
    pub links: Vec<String>,
}

pub fn fetch_and_parse(url: &str) -> Result<Page, Box<dyn std::error::Error>> {
    // Set a timeout and user agent
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (compatible; RustSearchBot/1.0)")
        .build()?;

    // Fetch HTML
    let response = client.get(url).send()?;
    let base_url = response.url().clone();
    let html_content = response.text()?;

    // Parse HTML
    let document = Html::parse_document(&html_content);

    // Extract title
    let title_selector = Selector::parse("title").unwrap();
    let title = document
        .select(&title_selector)
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or_else(|| "No title".to_string());

    // Extract body text (remove script and style tags)
    let body_selector = Selector::parse("body").unwrap();

    let body = document
        .select(&body_selector)
        .next()
        .map(|body_el| {
            let mut text = String::new();
            for node in body_el.descendants() {
                if let Some(text_node) = node.value().as_text() {
                    text.push_str(text_node);
                    text.push(' ');
                }
            }
            text.chars().filter(|c| !c.is_control()).collect::<String>()
        })
        .unwrap_or_default();

    // Extract all links
    let link_selector = Selector::parse("a[href]").unwrap();
    let links: Vec<String> = document
        .select(&link_selector)
        .filter_map(|el| {
            el.value()
                .attr("href")
                .and_then(|href| base_url.join(href).ok().map(|u| u.to_string()))
        })
        .collect();

    Ok(Page {
        url: url.to_string(),
        title: title.trim().to_string(),
        body: body.trim().to_string(),
        links,
    })
}

pub struct Crawler {
    visited: HashSet<String>,
    to_visit: Vec<String>,
    max_pages: usize,
}

impl Crawler {
    pub fn new(seed_urls: Vec<String>, max_pages: usize) -> Self {
        Crawler {
            visited: HashSet::new(),
            to_visit: seed_urls,
            max_pages,
        }
    }

    pub fn crawl<F>(&mut self, mut callback: F)
    where
        F: FnMut(&Page),
    {
        while let Some(url) = self.to_visit.pop() {
            // Stop if we've reached max pages
            if self.visited.len() >= self.max_pages {
                break;
            }

            // Skip if already visited
            if self.visited.contains(&url) {
                continue;
            }

            // Mark as visited
            self.visited.insert(url.clone());

            // Fetch and parse
            match fetch_and_parse(&url) {
                Ok(page) => {
                    // Call the callback with the page
                    callback(&page);

                    // Be polite - don't hammer servers (reduced to 300ms for faster crawling)
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
                Err(e) => {
                    // Only show errors occasionally to reduce noise
                    if self.visited.len() % 100 == 0 {
                        eprintln!("Failed to crawl {}: {}", url, e);
                    }
                }
            }
        }

        println!(
            "\n🎉 Crawling session complete! Visited {} pages",
            self.visited.len()
        );
    }
}
