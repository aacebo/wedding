use std::fs;

use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

#[derive(Template)]
#[template(path = "story.html")]
struct StoryPage {
    images: Vec<String>,
}

fn story_images() -> Vec<String> {
    let mut names: Vec<String> = match fs::read_dir("bins/api/assets/story") {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| {
                let lower = name.to_ascii_lowercase();
                matches!(
                    lower.rsplit('.').next(),
                    Some("jpg" | "jpeg" | "png" | "webp" | "gif")
                )
            })
            .collect(),
        Err(_) => Vec::new(),
    };

    names.sort();
    names
        .into_iter()
        .map(|name| format!("/assets/story/{name}"))
        .collect()
}

#[get("/story")]
pub async fn get() -> Result<Html, Error> {
    let page = StoryPage {
        images: story_images(),
    };

    Ok(Html::new(page.render().map_err(ErrorInternalServerError)?))
}
