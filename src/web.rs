use crate::indexer::SearchIndex;
use axum::{
    Router,
    extract::{Query, State},
    response::Html,
    routing::get,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    fuzzy: Option<String>,
}

async fn search_handler(
    State(index): State<Arc<SearchIndex>>,
    Query(params): Query<SearchQuery>,
) -> Html<String> {
    let query = params.q.unwrap_or_default();
    let use_fuzzy = params.fuzzy.as_deref() == Some("on");

    let results = if !query.is_empty() {
        if use_fuzzy {
            // Fuzzy search with edit distance of 2
            index.fuzzy_search(&query, 20, 2).unwrap_or_default()
        } else {
            // Regular search
            index.search(&query, 20).unwrap_or_default()
        }
    } else {
        Vec::new()
    };

    let total_docs = index.count_documents().unwrap_or(0);

    let results_html: String = if results.is_empty() && !query.is_empty() {
        r#"<div class="no-results">No results found. Try enabling fuzzy search or different keywords.</div>"#.to_string()
    } else {
        results
            .iter()
            .map(|(title, url, score)| {
                format!(
                    r#"<div class="result">
                        <a href="{}" target="_blank" class="result-title">{}</a>
                        <div class="result-url">{}</div>
                        <div class="result-score">Relevance: {:.2}</div>
                    </div>"#,
                    url,
                    html_escape(title),
                    html_escape(url),
                    score
                )
            })
            .collect()
    };

    let result_count = if !query.is_empty() {
        format!(
            "<p class='result-count'>Found {} results</p>",
            results.len()
        )
    } else {
        String::new()
    };

    let fuzzy_checked = if use_fuzzy { "checked" } else { "" };

    Html(format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Rust Search Engine</title>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            padding: 20px;
        }}
        .container {{
            max-width: 900px;
            margin: 0 auto;
            background: white;
            padding: 40px;
            border-radius: 15px;
            box-shadow: 0 10px 40px rgba(0,0,0,0.2);
        }}
        h1 {{
            color: #333;
            margin-bottom: 10px;
            font-size: 2.8em;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}
        .stats {{
            color: #666;
            margin-bottom: 25px;
            font-size: 0.95em;
        }}
        form {{
            margin-bottom: 30px;
        }}
        .search-box {{
            display: flex;
            margin-bottom: 15px;
        }}
        input[type="text"] {{
            flex: 1;
            padding: 15px 20px;
            font-size: 17px;
            border: 2px solid #ddd;
            border-radius: 8px 0 0 8px;
            outline: none;
            transition: border-color 0.3s;
        }}
        input[type="text"]:focus {{
            border-color: #667eea;
        }}
        button {{
            padding: 15px 35px;
            font-size: 17px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            border-radius: 0 8px 8px 0;
            cursor: pointer;
            transition: transform 0.2s, box-shadow 0.3s;
            font-weight: 600;
        }}
        button:hover {{
            transform: translateY(-2px);
            box-shadow: 0 5px 15px rgba(102, 126, 234, 0.4);
        }}
        button:active {{
            transform: translateY(0);
        }}
        .options {{
            display: flex;
            align-items: center;
            gap: 10px;
            color: #666;
            font-size: 0.95em;
        }}
        .checkbox-container {{
            display: flex;
            align-items: center;
            gap: 8px;
            background: #f8f9fa;
            padding: 8px 15px;
            border-radius: 6px;
            cursor: pointer;
            transition: background 0.3s;
        }}
        .checkbox-container:hover {{
            background: #e9ecef;
        }}
        input[type="checkbox"] {{
            width: 18px;
            height: 18px;
            cursor: pointer;
        }}
        .fuzzy-info {{
            font-size: 0.85em;
            color: #888;
            font-style: italic;
        }}
        .result {{
            padding: 20px 0;
            border-bottom: 1px solid #eee;
        }}
        .result:last-child {{
            border-bottom: none;
        }}
        .result-title {{
            color: #1a0dab;
            font-size: 20px;
            text-decoration: none;
            display: block;
            margin-bottom: 5px;
            font-weight: 500;
        }}
        .result-title:hover {{
            text-decoration: underline;
        }}
        .result-url {{
            color: #006621;
            font-size: 14px;
            overflow: hidden;
            text-overflow: ellipsis;
            white-space: nowrap;
            margin-bottom: 5px;
        }}
        .result-score {{
            color: #999;
            font-size: 12px;
            font-style: italic;
        }}
        .result-count {{
            color: #666;
            margin-bottom: 20px;
            font-size: 0.95em;
            padding: 10px;
            background: #f8f9fa;
            border-radius: 6px;
        }}
        .no-results {{
            color: #666;
            padding: 40px 20px;
            text-align: center;
            font-size: 1.1em;
            background: #f8f9fa;
            border-radius: 8px;
        }}
        .footer {{
            margin-top: 40px;
            padding-top: 20px;
            border-top: 1px solid #eee;
            text-align: center;
            color: #999;
            font-size: 0.85em;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🔍 Rust Search Engine</h1>
        <div class="stats">📚 Indexing {} websites • Powered by Tantivy & Rust</div>
        <form action="/" method="get">
            <div class="search-box">
                <input type="text" name="q" value="{}" placeholder="Search the web..." autofocus>
                <button type="submit">Search</button>
            </div>
            <div class="options">
                <label class="checkbox-container">
                    <input type="checkbox" name="fuzzy" {}>
                    <span>Enable Fuzzy Search</span>
                </label>
                <span class="fuzzy-info">(finds similar words, handles typos)</span>
            </div>
        </form>
        {}
        <div class="results">{}</div>
        <div class="footer">
            Built with 🦀 Rust • Tantivy Search Engine • Majestic Million Dataset
        </div>
    </div>
</body>
</html>
    "#,
        total_docs,
        html_escape(&query),
        fuzzy_checked,
        result_count,
        results_html
    ))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub async fn run_server(index: Arc<SearchIndex>) {
    let app = Router::new()
        .route("/", get(search_handler))
        .with_state(index);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();

    println!("\n🚀 Server running on http://127.0.0.1:3000");
    println!("✨ Features: Regular Search + Fuzzy Search");
    println!("Press Ctrl+C to stop\n");

    axum::serve(listener, app).await.unwrap();
}
