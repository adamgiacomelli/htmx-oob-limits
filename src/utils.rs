pub fn rgb_to_rounded_hex_color_string(c: [u8; 3]) -> String {
    let f = 10.0;
    let rounded_r = round_to_nearest(c[0], f);
    let rounded_g = round_to_nearest(c[1], f);
    let rounded_b = round_to_nearest(c[2], f);

    format!("#{:02X}{:02X}{:02X}", rounded_r, rounded_g, rounded_b)
}

fn round_to_nearest(n: u8, factor: f32) -> u8 {
    ((n as f32 / factor).round() * factor) as u8
}
