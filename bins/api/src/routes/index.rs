use actix_web::{Error, error::ErrorInternalServerError, get, web::Html};
use askama::Template;

use crate::RequestContext;

#[derive(Template)]
#[template(path = "index/page.html")]
struct IndexPage {
    version: &'static str,
    start_time: String,
    uptime: String,
    request_id: String,
}

#[get("/")]
pub async fn get(ctx: RequestContext) -> Result<Html, Error> {
    let page = IndexPage {
        version: env!("CARGO_PKG_VERSION"),
        start_time: ctx.start_time().to_rfc3339(),
        uptime: format_uptime(ctx.uptime()),
        request_id: ctx.request_id().to_string(),
    };

    Ok(Html::new(page.render().map_err(ErrorInternalServerError)?))
}

fn format_uptime(d: chrono::Duration) -> String {
    let total = d.num_seconds().max(0);
    let days = total / 86_400;
    let hours = (total % 86_400) / 3_600;
    let minutes = (total % 3_600) / 60;
    let seconds = total % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m {seconds}s")
    } else if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}
