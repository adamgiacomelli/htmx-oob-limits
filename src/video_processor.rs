use ffmpeg::software::scaling::{context::Context as SwsContext, flag::Flags};
use ffmpeg_next as ffmpeg;
use image::{DynamicImage, GenericImageView, ImageBuffer, Pixel, Rgb};
// Function to extract frames from the video and process them
pub fn extract_and_process_frames(
    video_path: &str,
    n: usize,
) -> Result<Vec<Vec<Vec<[u8; 3]>>>, ffmpeg::Error> {
    ffmpeg::init()?; // Initialize FFmpeg

    let mut ictx = ffmpeg::format::input(&video_path)?;

    // Find the best video stream in the input file
    let input = ictx
        .streams()
        .best(ffmpeg::media::Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;

    let video_stream_index = input.index();

    // Get codec parameters from the video stream
    let codec_params = input.parameters();

    // Create a codec context from the codec parameters
    let codec_context = ffmpeg::codec::Context::from_parameters(codec_params)?;

    // Create the video decoder
    let mut decoder = codec_context.decoder().video()?;

    // Create a frame to store the decoded data
    let mut frame = ffmpeg::util::frame::Video::empty();

    // Create a scaling context to convert the frame from YUV to RGB
    let mut scaler = SwsContext::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        ffmpeg_next::format::Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )?;

    // Create a destination frame for the RGB data
    let mut rgb_frame = ffmpeg::util::frame::Video::empty();
    rgb_frame.set_format(ffmpeg_next::format::Pixel::RGB24);
    rgb_frame.set_width(decoder.width());
    rgb_frame.set_height(decoder.height());

    let mut fgrid: Vec<Vec<Vec<[u8; 3]>>> = Vec::new();

    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;

            // Decode frames and store them in the `frame`
            while decoder.receive_frame(&mut frame).is_ok() {
                // Use the scaler to convert the frame to RGB
                scaler.run(&frame, &mut rgb_frame)?;

                // Convert the RGB frame to an ImageBuffer for processing
                let img_buffer: ImageBuffer<Rgb<u8>, _> = ImageBuffer::from_raw(
                    rgb_frame.width(),
                    rgb_frame.height(),
                    rgb_frame.data(0).to_vec(),
                )
                .unwrap();

                let dynamic_image = DynamicImage::ImageRgb8(img_buffer);

                // Process the frame into a grid of average colors
                let grid = process_frame_to_grid(dynamic_image, n);

                // Print the grid (or save it, etc.)
                // println!("Frame grid: {:?}", grid);
                fgrid.push(grid);
            }
        }
    }

    decoder.send_eof()?;

    Ok(fgrid)
}
fn process_frame_to_grid(frame: DynamicImage, n_s: usize) -> Vec<Vec<[u8; 3]>> {
    let n: u32 = n_s.try_into().unwrap();
    let (width, height) = frame.dimensions();
    let tile_width = width / n;
    let tile_height = height / n;

    let mut grid = Vec::new();

    for y in 0..n {
        let mut row = Vec::new();
        for x in 0..n {
            let mut r_sum = 0u64;
            let mut g_sum = 0u64;
            let mut b_sum = 0u64;
            let mut pixel_count = 0u64;

            // Loop through the pixels in the tile
            for i in (x * tile_width)..((x + 1) * tile_width) {
                for j in (y * tile_height)..((y + 1) * tile_height) {
                    let pixel = frame.get_pixel(i, j).to_rgb();
                    r_sum += pixel[0] as u64;
                    g_sum += pixel[1] as u64;
                    b_sum += pixel[2] as u64;
                    pixel_count += 1;
                }
            }

            // Calculate the average color
            let avg_color = [
                (r_sum / pixel_count) as u8,
                (g_sum / pixel_count) as u8,
                (b_sum / pixel_count) as u8,
            ];

            row.push(avg_color);
        }
        grid.push(row);
    }

    grid
}

