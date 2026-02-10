pub mod crawler;
pub mod indexer;
pub mod web;
use crate::indexer::SearchIndex;
use anyhow::Result;
use std::sync::Arc;

async fn download_all_top_sites() -> Result<Vec<String>> {
    println!("📥 Downloading ALL domains from Majestic Million...");

    let csv_data = reqwest::get("https://downloads.majestic.com/majestic_million.csv")
        .await?
        .text()
        .await?;

    let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
    let mut urls = Vec::new();

    for result in reader.records() {
        if let Ok(record) = result {
            if let Some(domain) = record.get(2) {
                urls.push(format!("https://{}", domain));
            }
        }
    }

    println!("✅ Downloaded {} URLs from Majestic Million\n", urls.len());
    Ok(urls)
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🦀 Rust Search Engine - Full Web Crawler Edition\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Create or open index
    let index = Arc::new(indexer::SearchIndex::create_or_open("./search_index")?);

    // Check if we already have indexed pages
    let existing_docs = index.count_documents()?;

    if existing_docs > 0 {
        println!("📚 Found existing index with {} documents", existing_docs);
        println!("\nOptions:");
        println!("  1. Use existing index and start server (recommended)");
        println!("  2. Continue crawling more sites");
        println!("  3. Delete and rebuild from scratch");
        println!("\nEnter choice (1, 2, or 3): ");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        match input.trim() {
            "2" => {
                crawl_and_index(index.clone(), false).await?;
            }
            "3" => {
                println!("\n🗑️  Removing old index...");
                std::fs::remove_dir_all("./search_index")?;
                let index = Arc::new(indexer::SearchIndex::create_or_open("./search_index")?);
                crawl_and_index(index.clone(), true).await?;
            }
            _ => {
                println!("\n✅ Using existing index with {} documents", existing_docs);
            }
        }
    } else {
        crawl_and_index(index.clone(), true).await?;
    }

    // Start web server
    web::run_server(index).await;

    Ok(())
}

async fn crawl_and_index(index: Arc<SearchIndex>, is_new: bool) -> Result<()> {
    println!("🕷️  Starting crawler...\n");

    // Download ALL available URLs from Majestic Million
    let urls = download_all_top_sites().await?;

    if is_new {
        println!(
            "🎯 Target: Crawl ALL {} websites from Majestic Million",
            urls.len()
        );
        println!("⚠️  WARNING: This will take a VERY long time (days/weeks)!");
        println!("💡 TIP: You can stop anytime with Ctrl+C - progress is saved automatically\n");
        println!("Recommended: Start with a smaller batch first by editing main.rs");
        println!("\nDo you want to proceed? (yes/no): ");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() != "yes" {
            println!("\n💡 Tip: To start smaller, change this line in main.rs:");
            println!("   let urls = download_all_top_sites().await?;");
            println!("   to:");
            println!("   let urls = download_top_sites(10000).await?; // 10k sites\n");
            return Ok(());
        }
    }

    println!("🔍 Crawling {} websites...", urls.len());
    println!("Progress will be shown every 50 pages.\n");

    let index_clone = index.clone();

    // Run crawler in blocking thread
    tokio::task::spawn_blocking(move || {
        let mut crawler = crawler::Crawler::new(urls, usize::MAX); // No limit

        let mut successful = 0;
        let mut failed = 0;
        let start_time = std::time::Instant::now();

        crawler.crawl(|page| {
            // Only index if we got meaningful content
            if !page.title.is_empty() && page.body.len() > 100 {
                if let Err(e) = index_clone.add_document(&page.title, &page.body, &page.url) {
                    eprintln!("❌ Failed to index {}: {}", page.url, e);
                    failed += 1;
                } else {
                    successful += 1;

                    // Show progress every 50 pages
                    if successful % 50 == 0 {
                        let elapsed = start_time.elapsed();
                        let rate = successful as f64 / elapsed.as_secs() as f64;
                        println!(
                            "📊 Progress: {} indexed, {} failed | Rate: {:.1} pages/sec",
                            successful, failed, rate
                        );
                    }
                }
            }
        });

        let total_time = start_time.elapsed();
        println!("\n✅ Crawling complete!");
        println!("   Successfully indexed: {}", successful);
        println!("   Failed: {}", failed);
        println!(
            "   Total time: {:.2} hours",
            total_time.as_secs() as f64 / 3600.0
        );
        println!(
            "   Average: {:.2} pages/second",
            successful as f64 / total_time.as_secs() as f64
        );
    })
    .await?;

    Ok(())
}

async fn download_top_sites(limit: usize) -> Result<Vec<String>> {
    println!("📥 Downloading Majestic Million top {} domains...", limit);

    let csv_data = reqwest::get("https://downloads.majestic.com/majestic_million.csv")
        .await?
        .text()
        .await?;

    let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
    let mut urls = Vec::new();

    for result in reader.records().take(limit) {
        if let Ok(record) = result {
            if let Some(domain) = record.get(2) {
                urls.push(format!("https://{}", domain));
            }
        }
    }

    println!("✅ Downloaded {} URLs\n", urls.len());
    Ok(urls)
}
