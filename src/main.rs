use actix_files as fs;
use actix_web::{get, middleware::Logger, web, App, HttpResponse, HttpServer, Responder};
use broadcast::Broadcaster;
use hex_color::{Display, HexColor};
use maud::{html, DOCTYPE};
use rand::Rng;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::interval;
use video_processor::{extract_and_process_frames, generate_grid, Tile};
mod broadcast;
mod video_processor;

struct AppState {
    app_name: String,
    grid: Vec<Vec<Tile>>,
    frame_data: Vec<Vec<Vec<[u8; 3]>>>,
    broadcaster: Arc<Broadcaster>,
}

static GRID_SIZE: usize = 15;
static SIZE_RECT: i32 = 10;
static RANDOM: bool = false;

fn to_hex_color(c: [u8; 3]) -> String {
    // Helper function to round numbers to the nearest 10
    fn round_to_nearest_ten(n: u8) -> u8 {
        // n
        ((n as f32 / 10.0).round() * 10.0) as u8
    }

    // Applying rounding to each color component
    let rounded_r = round_to_nearest_ten(c[0]);
    let rounded_g = round_to_nearest_ten(c[1]);
    let rounded_b = round_to_nearest_ten(c[2]);

    format!("#{:02X}{:02X}{:02X}", rounded_r, rounded_g, rounded_b)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let video_path = "example.mp4";
    let frame_data = extract_and_process_frames(video_path, GRID_SIZE)?;

    let state = Arc::new(AppState {
        app_name: String::from("HTMX grid oob"),
        grid: generate_grid(GRID_SIZE),
        frame_data,
        broadcaster: Broadcaster::create(),
    });
    let state_clone = Arc::clone(&state);

    let is_running = Arc::new(AtomicBool::new(false));
    actix_rt::spawn(async move {
        if is_running.load(Ordering::SeqCst) {
            return;
        }

        is_running.store(true, Ordering::SeqCst);

        let mut rng_gen = rand::thread_rng();
        let mut interval = interval(Duration::from_millis(60));
        let mut frame = 0;
        let mut last_frame: Vec<Vec<[u8; 3]>> = Vec::new();
        loop {
            interval.tick().await;

            if RANDOM {
                let random_rgb: HexColor = rand::random();
                let html_color_str = Display::new(random_rgb);

                let id_row = rng_gen.gen_range(0..GRID_SIZE);
                let id_col = rng_gen.gen_range(0..GRID_SIZE);
                let id = format!("{id_row}_{id_col}");
                let body = html! {
                    #{"t"(id)} hx-swap-oob="true" style={"background:"(html_color_str)} { (id) }
                };

                state_clone.broadcaster.broadcast(&body.into_string()).await;
            } else {
                for (id_row, r) in state_clone.frame_data[frame].iter().enumerate() {
                    let mut frame_update: String = Default::default();
                    for (id_col, c) in r.iter().enumerate() {
                        if !last_frame.is_empty() {
                            // Convert the RGB array (c) into a hex color string
                            let html_color_str = to_hex_color(*c);
                            if html_color_str != to_hex_color(last_frame[id_row][id_col]) {
                                // Create the tile id using the row and column indices
                                let id = format!("{id_row}_{id_col}");

                                // Generate the HTML body
                                let body = html! {
                                    #{"t"(id)} style={"top:"(id_row*SIZE_RECT as usize)"px; left:"(id_col*SIZE_RECT as usize)"px;background-color:"(html_color_str)";"} hx-swap-oob="true"  {}
                                };
                                frame_update += &body.into_string();
                                // state_clone.broadcaster.broadcast(&body.into_string()).await;
                            }
                        }
                    }
                    // Broadcast the HTML to update the front end
                    state_clone.broadcaster.broadcast(&frame_update).await;
                }

                last_frame = state_clone.frame_data[frame].clone();
                frame += 1;
                if frame >= state_clone.frame_data.len() {
                    frame = 0;
                }
            }

            // println!("Color on {} updated to {}", id, html_color_str);
            println!("Iteration done.");
            // Reset the flag to false when done
            is_running.store(false, Ordering::SeqCst);
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
                .wrapper {
                    @for row in &state.grid {
                        @for col in row {
                           #{"t"(col.id)} style={"left:"(col.x*SIZE_RECT)"px; top:"(col.y*SIZE_RECT)"px;"}  { }
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
