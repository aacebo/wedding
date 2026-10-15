use std::fs;

use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

use super::PageMetadata;

#[derive(Template)]
#[template(path = "story.html")]
struct StoryPage {
    meta: PageMetadata,
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
        meta: PageMetadata::new(
            "Our Love Story — Nancy & Alexander",
            "One first date, one wine-fueled text, and a spontaneous trip to Italy. The love story of Nancy and Alexander.",
            "https://baicebo.com/story",
        ),
        images: story_images(),
    };

    Ok(Html::new(page.render().map_err(ErrorInternalServerError)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{App, test};

    #[actix_web::test]
    async fn story_renders_approved_copy_without_placeholder_notices() {
        let app = test::init_service(App::new().service(get)).await;
        let response =
            test::call_service(&app, test::TestRequest::get().uri("/story").to_request()).await;
        assert!(response.status().is_success());
        let body = String::from_utf8(test::read_body(response).await.to_vec()).unwrap();
        for text in [
            "Our Love Story",
            "one wine-fueled text",
            "The next morning, Nancy asked again.",
            "And Alex said yes.",
            "this time with their favorite people.",
            "their favorite one yet.",
        ] {
            assert!(body.contains(text), "Missing story text: {text}");
        }
        assert!(!body.contains("Full story coming soon"));
        assert!(!body.contains("More details coming soon"));
        assert!(!body.contains("aria-label=\"Page status\""));
    }
}
