use actix_files as fs;
use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use broadcast::Broadcaster;
use hex_color::{Display, HexColor};
use maud::{html, DOCTYPE};
use rand::Rng;
use std::{sync::Arc, time::Duration};
use tokio::time::interval;
mod broadcast;

struct Tile {
    id: String,
    color: String,
}
struct AppState {
    app_name: String,
    grid: Vec<Vec<Tile>>,
    broadcaster: Arc<Broadcaster>,
}

static GRID_SIZE: usize = 20;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let state = Arc::new(AppState {
        app_name: String::from("HTMX grid oob"),
        grid: generate_grid(GRID_SIZE),
        broadcaster: Broadcaster::create(),
    });
    let state_clone = Arc::clone(&state);

    actix_rt::spawn(async move {
        let mut rng_gen = rand::thread_rng();
        let mut interval = interval(Duration::from_millis(16));

        loop {
            interval.tick().await;
            let random_rgb: HexColor = rand::random();
            let html_color_str = Display::new(random_rgb);

            let id_row = rng_gen.gen_range(0..GRID_SIZE);
            let id_col = rng_gen.gen_range(0..GRID_SIZE);
            let id = format!("{id_row}_{id_col}");

            let body = html! {
                span #{"tile_"(id)} .tile hx-swap-oob="true" style={"background:"(html_color_str)} { (id) }
            };

            state_clone.broadcaster.broadcast(&body.into_string()).await;
            // println!("Color on {} updated to {}", id, html_color_str);
        }
    });

    log::info!("starting HTTP server at http://localhost:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::from(Arc::clone(&state)))
            .service(index)
            .service(event_stream)
            .service(data)
            .service(fs::Files::new("/", "./public"))
            .wrap(Logger::default())
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

fn generate_grid(n: usize) -> Vec<Vec<Tile>> {
    (0..n)
        .map(|x| {
            (0..n)
                .map(|y| Tile {
                    id: format!("{}_{}", x, y),
                    color: "blue".to_string(),
                })
                .collect()
        })
        .collect()
}

#[get("/")]
async fn index(state: web::Data<AppState>) -> impl Responder {
    let body = html! {
        (DOCTYPE)
        html {
            head {
                title  { (state.app_name) }
                link rel="stylesheet" href="style.css" {}
                script src="https://unpkg.com/htmx.org@2.0.3" {}
                script src="https://unpkg.com/htmx-ext-sse@2.2.2/sse.js" {}
            }
            body {
                h1 { "Htmx OOB grid" }
                div hx-get="/data" hx-trigger="load" {}
                div.wrapper {
                    @for row in &state.grid {
                        .row {
                            @for col in row {
                               div #{"tile_"(col.id)} .tile style={"background:"(col.color)} { (col.id) }
                            }
                        }
                    }
                }
                div hx-ext="sse" sse-connect="/events" sse-swap="message" {}
            }
        }
    };

    HttpResponse::Ok()
        .content_type("text/html")
        .body(body.into_string())
}

#[get("/events")]
async fn event_stream(state: web::Data<AppState>) -> impl Responder {
    state.broadcaster.new_client().await
}

#[get("/data")]
async fn data() -> impl Responder {
    let body = html! {
        span #"tile_2_4" .tile hx-swap-oob="true" style={"background:red"} { "2_4" }
    };

    HttpResponse::Ok().body(body.into_string())
}
