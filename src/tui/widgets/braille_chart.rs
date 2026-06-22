use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use rust_decimal::Decimal;

/// Braille dot bit for a 2×4 sub-cell (left column x=0, right column x=1).
fn dot_bit(x: usize, y: usize) -> u8 {
    match (x, y) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (0, 3) => 0x08,
        (1, 0) => 0x10,
        (1, 1) => 0x20,
        (1, 2) => 0x40,
        (1, 3) => 0x80,
        _ => 0,
    }
}

fn set_pixel(grid: &mut [u8], char_width: usize, height_px: usize, x: usize, y: usize) {
    let char_x = x / 2;
    let char_y = y / 4;
    let idx = char_y * char_width + char_x;
    if idx < grid.len() {
        grid[idx] |= dot_bit(x % 2, y % 4);
    }
    let _ = height_px;
}

fn draw_line(grid: &mut [u8], char_width: usize, height_px: usize, x0: i32, y0: i32, x1: i32, y1: i32) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;

    loop {
        if x >= 0 && y >= 0 {
            set_pixel(grid, char_width, height_px, x as usize, y as usize);
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_string().parse().unwrap_or(0.0)
}

fn resample(values: &[f64], target_len: usize) -> Vec<f64> {
    if target_len == 0 {
        return Vec::new();
    }
    if values.is_empty() {
        return vec![0.0; target_len];
    }
    if values.len() == 1 {
        return vec![values[0]; target_len];
    }
    let last = values.len() - 1;
    (0..target_len)
        .map(|i| {
            let pos = i as f64 * last as f64 / (target_len - 1).max(1) as f64;
            let lo = pos.floor() as usize;
            let hi = pos.ceil() as usize;
            if lo == hi {
                values[lo]
            } else {
                let frac = pos - lo as f64;
                values[lo] * (1.0 - frac) + values[hi] * frac
            }
        })
        .collect()
}

/// Renders a braille line chart from close prices.
pub fn render_lines(closes: &[Decimal], width: u16, height: u16) -> Vec<Line<'static>> {
    let width = width.max(1) as usize;
    let height = height.max(1) as usize;
    let width_px = width * 2;
    let height_px = height * 4;

    if closes.is_empty() {
        return vec![Line::from(Span::styled(
            "—".repeat(width),
            Style::default().fg(Color::DarkGray),
        ))];
    }

    let values: Vec<f64> = closes.iter().map(|d| decimal_to_f64(*d)).collect();
    let resampled = resample(&values, width_px);
    let min = resampled
        .iter()
        .copied()
        .fold(f64::INFINITY, f64::min);
    let max = resampled
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let range = if (max - min).abs() < f64::EPSILON {
        1.0
    } else {
        max - min
    };

    let mut grid = vec![0u8; width * height];
    let mut prev: Option<(i32, i32)> = None;
    for (i, &v) in resampled.iter().enumerate() {
        let norm = (v - min) / range;
        let y = (height_px.saturating_sub(1) as f64 - norm * (height_px.saturating_sub(1) as f64))
            .round() as i32;
        let x = i as i32;
        if let Some((px, py)) = prev {
            draw_line(&mut grid, width, height_px, px, py, x, y);
        } else {
            set_pixel(&mut grid, width, height_px, x as usize, y as usize);
        }
        prev = Some((x, y));
    }

    let style = Style::default().fg(Color::Cyan);
    grid.chunks(width)
        .map(|row| {
            let s: String = row
                .iter()
                .map(|bits| char::from_u32(0x2800 + *bits as u32).unwrap_or(' '))
                .collect();
            Line::from(Span::styled(s, style))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn renders_non_empty_chart() {
        let closes: Vec<Decimal> = (0..30)
            .map(|i| dec!(20) + Decimal::from(i) / Decimal::from(10))
            .collect();
        let lines = render_lines(&closes, 40, 4);
        assert_eq!(lines.len(), 4);
        let text: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.clone()))
            .collect();
        assert!(text.chars().any(|c| c as u32 >= 0x2800));
    }

    #[test]
    fn empty_input_shows_placeholder() {
        let lines = render_lines(&[], 20, 3);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].spans[0].content.contains('—'));
    }
}
