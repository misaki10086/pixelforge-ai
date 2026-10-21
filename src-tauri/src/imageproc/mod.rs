//! 图像处理管线：每个步骤都是独立函数，可组合、可测试。
//! 第一阶段实现：背景移除（边沿洪泛）、透明清理、最近邻缩放（像素化对齐）、缩略图。

use image::{imageops::FilterType, Rgba, RgbaImage};

/// 从四边向内洪泛移除与边缘底色相近的像素（保护画面内部同色区域）
pub fn remove_background(img: &RgbaImage, tolerance: i32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return img.clone();
    }
    let mut out = img.clone();

    // 参考底色：四角像素均值
    let corners = [
        *out.get_pixel(0, 0),
        *out.get_pixel(w - 1, 0),
        *out.get_pixel(0, h - 1),
        *out.get_pixel(w - 1, h - 1),
    ];
    let mut r = 0i32;
    let mut g = 0i32;
    let mut b = 0i32;
    for c in &corners {
        r += c[0] as i32;
        g += c[1] as i32;
        b += c[2] as i32;
    }
    let bg = Rgba([
        (r / 4) as u8,
        (g / 4) as u8,
        (b / 4) as u8,
        255,
    ]);

    let near = |p: &Rgba<u8>| -> bool {
        let dr = (p[0] as i32 - bg[0] as i32).abs();
        let dg = (p[1] as i32 - bg[1] as i32).abs();
        let db = (p[2] as i32 - bg[2] as i32).abs();
        dr <= tolerance && dg <= tolerance && db <= tolerance
    };

    let mut visited = vec![false; (w * h) as usize];
    let mut queue = std::collections::VecDeque::new();
    for x in 0..w {
        queue.push_back((x, 0));
        queue.push_back((x, h - 1));
    }
    for y in 0..h {
        queue.push_back((0, y));
        queue.push_back((w - 1, y));
    }

    while let Some((x, y)) = queue.pop_front() {
        let idx = (y * w + x) as usize;
        if visited[idx] {
            continue;
        }
        visited[idx] = true;
        let p = out.get_pixel_mut(x, y);
        if near(p) {
            p[3] = 0;
            if x > 0 {
                queue.push_back((x - 1, y));
            }
            if x < w - 1 {
                queue.push_back((x + 1, y));
            }
            if y > 0 {
                queue.push_back((x, y - 1));
            }
            if y < h - 1 {
                queue.push_back((x, y + 1));
            }
        }
    }
    out
}

/// 透明度二值化：像素画适用硬边透明（低于阈值全透明，其余全不透明）
pub fn cleanup_alpha(img: &RgbaImage, threshold: u8) -> RgbaImage {
    let mut out = img.clone();
    for p in out.pixels_mut() {
        p[3] = if p[3] < threshold { 0 } else { 255 };
    }
    out
}

/// 最近邻缩放：保持硬边像素，用于把 AI 输出对齐到目标分辨率
pub fn resize_nearest(img: &RgbaImage, target: u32) -> RgbaImage {
    if img.width() == target && img.height() == target {
        return img.clone();
    }
    image::imageops::resize(img, target, target, FilterType::Nearest)
}

/// 生成缩略图（长边 256，最近邻保持像素锐利）
pub fn make_thumbnail(img: &RgbaImage, max: u32) -> RgbaImage {
    let scale = f64::from(max) / f64::from(img.width().max(img.height()));
    if scale >= 1.0 {
        return img.clone();
    }
    let tw = ((f64::from(img.width()) * scale).round() as u32).max(1);
    let th = ((f64::from(img.height()) * scale).round() as u32).max(1);
    image::imageops::resize(img, tw, th, FilterType::Nearest)
}

// ---------------------------------------------------------------------------
// 颜色 / 调色板（第二阶段：调色 / 风格一致性）
// ---------------------------------------------------------------------------

/// 抖动模式：Bayer 4x4 有序抖动
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DitherMode {
    None,
    Bayer4,
}

const BAYER4: [[i32; 4]; 4] = [
    [0, 8, 2, 10],
    [12, 4, 14, 6],
    [3, 11, 1, 9],
    [15, 7, 13, 5],
];

pub type Rgb = [u8; 3];

fn channel_range(pixels: &[[u8; 3]], ch: usize) -> i32 {
    let (mut mn, mut mx) = (255i32, 0i32);
    for p in pixels {
        mn = mn.min(p[ch] as i32);
        mx = mx.max(p[ch] as i32);
    }
    mx - mn
}

/// 中位切分（Median Cut）：从采样像素中提取 k 个代表色
pub fn median_cut(samples: &[[u8; 3]], k: usize) -> Vec<Rgb> {
    if samples.is_empty() {
        return vec![];
    }
    let k = k.max(1);
    let mut boxes: Vec<Vec<[u8; 3]>> = vec![samples.to_vec()];
    while boxes.len() < k {
        // 找到通道跨度最大的箱子进行切分
        let mut best: Option<(usize, usize)> = None;
        let mut best_range: i32 = 0;
        for (i, b) in boxes.iter().enumerate() {
            if b.len() < 2 {
                continue;
            }
            for ch in 0..3 {
                let range = channel_range(b, ch);
                if range > best_range {
                    best_range = range;
                    best = Some((i, ch));
                }
            }
        }
        let Some((bi, ch)) = best else { break };
        let mut b = boxes.swap_remove(bi);
        b.sort_by_key(|p| p[ch]);
        let mid = b.len() / 2;
        boxes.push(b[..mid].to_vec());
        boxes.push(b[mid..].to_vec());
    }
    boxes
        .into_iter()
        .filter(|b| !b.is_empty())
        .map(|b| {
            let n = b.len() as i64;
            let mut sum = [0i64; 3];
            for p in &b {
                for c in 0..3 {
                    sum[c] += p[c] as i64;
                }
            }
            [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8]
        })
        .collect()
}

/// 采样不透明像素用于调色板提取（步进采样，最多约 2 万点）
pub fn sample_pixels(img: &RgbaImage, max_samples: usize) -> Vec<[u8; 3]> {
    let total = (img.width() * img.height()) as usize;
    let step = (total / max_samples.max(1)).max(1) as u32;
    let mut out = vec![];
    let mut i = 0u32;
    for p in img.pixels() {
        if i % step == 0 && p[3] >= 128 {
            out.push([p[0], p[1], p[2]]);
        }
        i += 1;
    }
    out
}

/// 从图像提取 k 个主色（返回 RGB）
pub fn extract_palette(img: &RgbaImage, k: usize) -> Vec<Rgb> {
    let samples = sample_pixels(img, 20000);
    median_cut(&samples, k)
}

fn nearest_color(palette: &[[u8; 3]], c: [i32; 3]) -> [u8; 3] {
    let mut best = palette[0];
    let mut best_d = i64::MAX;
    for p in palette {
        let d = (p[0] as i64 - c[0] as i64).pow(2)
            + (p[1] as i64 - c[1] as i64).pow(2)
            + (p[2] as i64 - c[2] as i64).pow(2);
        if d < best_d {
            best_d = d;
            best = *p;
        }
    }
    best
}

/// 把图像量化到给定调色板（保留二值透明），可选 Bayer 抖动
pub fn quantize_to_palette(img: &RgbaImage, palette: &[[u8; 3]], dither: DitherMode) -> RgbaImage {
    if palette.is_empty() {
        return img.clone();
    }
    let mut out = img.clone();
    let (w, _h) = out.dimensions();
    for (idx, p) in out.pixels_mut().enumerate() {
        if p[3] < 128 {
            p[3] = 0;
            continue;
        }
        let x = (idx as u32 % w) as usize;
        let y = (idx as u32 / w) as usize;
        let mut c = [p[0] as i32, p[1] as i32, p[2] as i32];
        if dither == DitherMode::Bayer4 {
            let t = (BAYER4[y % 4][x % 4] as f32 / 16.0 - 0.5) * 48.0;
            for ch in 0..3 {
                c[ch] = (c[ch] as f32 + t).round().clamp(0.0, 255.0) as i32;
            }
        }
        let q = nearest_color(palette, c);
        *p = Rgba([q[0], q[1], q[2], 255]);
    }
    out
}

/// 按调色板数量量化（先提取再套用），一步完成
pub fn quantize_colors(img: &RgbaImage, k: usize, dither: DitherMode) -> RgbaImage {
    let palette = extract_palette(img, k);
    quantize_to_palette(img, &palette, dither)
}

// ---------------------------------------------------------------------------
// Sprite Sheet 切帧
// ---------------------------------------------------------------------------

/// 把一张图按 cols × rows 均匀切分（行优先），用于拆分 AI 生成的横向 Sprite Sheet
pub fn split_image_cells(img: &RgbaImage, cols: u32, rows: u32) -> Vec<RgbaImage> {
    let (cols, rows) = (cols.max(1), rows.max(1));
    let cw = (img.width() / cols).max(1);
    let ch = (img.height() / rows).max(1);
    let mut cells = vec![];
    for r in 0..rows {
        for c in 0..cols {
            let cell = image::imageops::crop_imm(img, c * cw, r * ch, cw, ch).to_image();
            cells.push(cell);
        }
    }
    cells
}

/// 按透明列间隙智能切帧（单行 Sprite Sheet）。
///
/// 参考 Retro Diffusion / 各类 sprite slicer 的思路：AI 输出的帧间距往往不均匀，
/// 均分切帧会切坏角色。此处检测"整列全透明"作为帧分隔，把不透明列段分组为帧；
/// 若检测段数与期望帧数一致则采用，否则回退到均匀切分。
/// 相邻段之间极窄的间隙（如手持武器与身体的 1–2px 断缝）会按阈值合并，避免误切。
pub fn split_image_auto(img: &RgbaImage, expected_frames: u32) -> Vec<RgbaImage> {
    let (w, h) = img.dimensions();
    if w == 0 || h == 0 || expected_frames == 0 {
        return vec![];
    }
    let expected = expected_frames as usize;

    // 每列是否含不透明像素（阈值 16，抗半透明噪点）
    let col_opaque: Vec<bool> = (0..w)
        .map(|x| (0..h).any(|y| img.get_pixel(x, y)[3] >= 16))
        .collect();

    let mut runs: Vec<(u32, u32)> = Vec::new(); // [start, end)
    let mut x = 0u32;
    while x < w {
        if col_opaque[x as usize] {
            let start = x;
            while x < w && col_opaque[x as usize] {
                x += 1;
            }
            runs.push((start, x));
        } else {
            x += 1;
        }
    }

    // 合并窄间隙：阈值随期望帧宽自适应，至少 2px
    let min_gap = (((w as f32) / (expected_frames.max(1) as f32 * 10.0)).ceil() as u32).max(2);
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for &(start, end) in &runs {
        match merged.last_mut() {
            Some(last) if start - last.1 < min_gap => last.1 = end,
            _ => merged.push((start, end)),
        }
    }

    // 优先用合并后的段；若数量不符但原始段数恰好吻合（帧内有细断缝），用原始段
    let chosen: Vec<(u32, u32)> = if merged.len() == expected {
        merged
    } else if runs.len() == expected {
        runs
    } else {
        // 检测失败（无透明背景 / 段数不符）→ 回退均分
        return split_image_cells(img, expected_frames, 1);
    };
    chosen
        .iter()
        .map(|(s, e)| image::imageops::crop_imm(img, *s, 0, e - s, h).to_image())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, c: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(c))
    }

    #[test]
    fn remove_background_clears_border_region_only() {
        // 白底中央一个红色方块：只有连通到边缘的白底被移除
        let mut img = solid(16, 16, [255, 255, 255, 255]);
        for y in 6..10 {
            for x in 6..10 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        let out = remove_background(&img, 24);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        assert_eq!(out.get_pixel(8, 8)[3], 255); // 中央红块不受影响
        assert_eq!(out.get_pixel(8, 8)[0], 255);
    }

    #[test]
    fn cleanup_alpha_binarizes() {
        let mut img = solid(4, 4, [0, 0, 0, 128]);
        img.put_pixel(0, 0, Rgba([0, 0, 0, 5]));
        let out = cleanup_alpha(&img, 16);
        assert_eq!(out.get_pixel(0, 0)[3], 0);
        assert_eq!(out.get_pixel(2, 2)[3], 255);
    }

    #[test]
    fn resize_nearest_changes_size() {
        let img = solid(32, 32, [10, 20, 30, 255]);
        let out = resize_nearest(&img, 64);
        assert_eq!(out.dimensions(), (64, 64));
        assert_eq!(out.get_pixel(10, 10)[0], 10);
    }

    #[test]
    fn thumbnail_never_upscales() {
        let img = solid(32, 32, [0, 0, 0, 255]);
        assert_eq!(make_thumbnail(&img, 256).dimensions(), (32, 32));
        let big = solid(512, 512, [0, 0, 0, 255]);
        assert_eq!(make_thumbnail(&big, 256).dimensions(), (256, 256));
    }

    fn two_color_img() -> RgbaImage {
        let mut img = RgbaImage::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                let c = if x < 4 { [220, 40, 40, 255] } else { [40, 60, 220, 255] };
                img.put_pixel(x, y, Rgba(c));
            }
        }
        img
    }

    #[test]
    fn extract_palette_finds_dominant_colors() {
        let palette = extract_palette(&two_color_img(), 2);
        assert_eq!(palette.len(), 2);
        // 两个主色应分别接近红与蓝
        let has_red = palette.iter().any(|c| c[0] > 150 && c[2] < 120);
        let has_blue = palette.iter().any(|c| c[2] > 150 && c[0] < 120);
        assert!(has_red, "palette: {palette:?}");
        assert!(has_blue, "palette: {palette:?}");
    }

    #[test]
    fn quantize_maps_to_nearest_palette_color() {
        let palette: Vec<Rgb> = vec![[255, 0, 0], [0, 0, 255]];
        let mut img = RgbaImage::new(4, 4);
        img.put_pixel(0, 0, Rgba([250, 30, 20, 255])); // 近红
        img.put_pixel(1, 1, Rgba([10, 10, 240, 255])); // 近蓝
        img.put_pixel(2, 2, Rgba([0, 0, 0, 40])); // 半透明 → 全透明
        let out = quantize_to_palette(&img, &palette, DitherMode::None);
        assert_eq!(out.get_pixel(0, 0).0[..3], [255, 0, 0]);
        assert_eq!(out.get_pixel(1, 1).0[..3], [0, 0, 255]);
        assert_eq!(out.get_pixel(2, 2)[3], 0);
    }

    #[test]
    fn quantize_colors_limits_output_colors() {
        let mut img = RgbaImage::new(16, 16);
        for y in 0..16u8 {
            for x in 0..16u8 {
                img.put_pixel(x as u32, y as u32, Rgba([x * 16, y * 16, 128, 255]));
            }
        }
        let out = quantize_colors(&img, 8, DitherMode::None);
        let mut colors = std::collections::HashSet::new();
        for p in out.pixels() {
            colors.insert((p[0], p[1], p[2]));
        }
        assert!(colors.len() <= 8, "got {} colors", colors.len());
    }

    #[test]
    fn split_image_cells_slices_grid() {
        let mut img = RgbaImage::new(64, 16);
        img.put_pixel(1, 1, Rgba([255, 0, 0, 255]));
        img.put_pixel(17, 1, Rgba([0, 255, 0, 255]));
        let cells = split_image_cells(&img, 4, 1);
        assert_eq!(cells.len(), 4);
        assert_eq!(cells[0].dimensions(), (16, 16));
        assert_eq!(cells[0].get_pixel(1, 1)[0], 255);
        assert_eq!(cells[1].get_pixel(1, 1)[1], 255);
    }

    /// 4 个 8px 宽方块，间隔 8px，总 56x8
    fn four_sprite_sheet() -> RgbaImage {
        let mut img = RgbaImage::new(56, 8);
        for i in 0..4 {
            let base = i * 16;
            for x in 0..8u32 {
                for y in 0..8u32 {
                    img.put_pixel(base + x, y, Rgba([255, 0, 0, 255]));
                }
            }
        }
        img
    }

    #[test]
    fn split_image_auto_detects_transparent_gaps() {
        let img = four_sprite_sheet();
        // 帧间距均匀时也能按间隙正确切出
        let cells = split_image_auto(&img, 4);
        assert_eq!(cells.len(), 4);
        assert!(cells.iter().all(|c| c.width() == 8 && c.height() == 8));
    }

    #[test]
    fn split_image_auto_survives_thin_cracks_inside_sprite() {
        // 帧内部 1px 断缝（如漂浮的剑与手之间）不应被切成两帧
        let mut img = RgbaImage::new(24, 8);
        for x in 0..8u32 {
            for y in 0..8u32 {
                img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            }
        }
        for x in 9..24u32 {
            for y in 0..8u32 {
                img.put_pixel(x, y, Rgba([0, 255, 0, 255]));
            }
        }
        let cells = split_image_auto(&img, 2);
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].width(), 8); // 第一帧只有红块
        assert_eq!(cells[1].width(), 15); // 第二帧合并了 1px 断缝后的绿块
    }

    #[test]
    fn split_image_auto_falls_back_when_no_gaps() {
        // 全不透明图：无法检测间隙，回退均分
        let img = solid(64, 16, [255, 0, 0, 255]);
        let cells = split_image_auto(&img, 4);
        assert_eq!(cells.len(), 4);
        assert!(cells.iter().all(|c| c.width() == 16));
    }
}
