use core::panic;
use hex_color::{Display, HexColor};
use maud::html;
use rand::Rng;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::time::interval;

use crate::{utils::rgb_to_rounded_hex_color_string, ServersideState};

pub fn start_sse_worker(
    mode: String,
    grid_size: usize,
    size_rect: i32,
    update_frequency: i32,
    state_clone: Arc<ServersideState>,
) {
    let is_running = Arc::new(AtomicBool::new(false));
    actix_rt::spawn(async move {
        if is_running.load(Ordering::SeqCst) {
            return;
        }

        is_running.store(true, Ordering::SeqCst);

        let mut rng_gen = rand::thread_rng();

        let delay = 1000.0 / update_frequency as f32;
        let mut interval = interval(Duration::from_millis(delay as u64));

        let mut frame = 0;
        let mut last_frame: Vec<Vec<[u8; 3]>> = Vec::new();
        loop {
            interval.tick().await;

            if mode == "random" {
                let random_rgb: HexColor = rand::random();
                let html_color_str = Display::new(random_rgb);

                let id_row = rng_gen.gen_range(0..grid_size);
                let id_col = rng_gen.gen_range(0..grid_size);
                let id = format!("{id_row}_{id_col}");
                let body = html! {
                    #{"t"(id)} style={"top:"(id_row*size_rect as usize)"px; left:"(id_col*size_rect as usize)"px;background-color:"(html_color_str)";"} hx-swap-oob="true"  {}
                };

                state_clone.broadcaster.broadcast(&body.into_string()).await;
            } else if mode == "video" {
                if let Some(frame_data) = &state_clone.frame_data {
                    for (id_row, r) in frame_data[frame].iter().enumerate() {
                        let mut frame_update: String = Default::default();
                        for (id_col, c) in r.iter().enumerate() {
                            if !last_frame.is_empty() {
                                let html_color_str = rgb_to_rounded_hex_color_string(*c);
                                if html_color_str
                                    != rgb_to_rounded_hex_color_string(last_frame[id_row][id_col])
                                {
                                    let id = format!("{id_row}_{id_col}");

                                    let body = html! {
                                        #{"t"(id)} style={"top:"(id_row*size_rect as usize)"px; left:"(id_col*size_rect as usize)"px;background-color:"(html_color_str)";"} hx-swap-oob="true"  {}
                                    };
                                    frame_update += &body.into_string();
                                }
                            }
                        }
                        // Broadcast the HTML to update the front end
                        state_clone.broadcaster.broadcast(&frame_update).await;
                    }

                    last_frame = frame_data[frame].clone();
                    frame += 1;
                    if frame >= frame_data.len() {
                        frame = 0;
                    }

                    println!("Iteration done.");
                    is_running.store(false, Ordering::SeqCst);
                } else {
                    panic!("Video frame data missing")
                }
            }
        }
    });
}
