#[inline]
pub fn soft_limit(x: f32) -> f32 {
    return x * (27.0 + x * x) / (27.0 + 9.0 * x * x);
}

#[inline]
pub fn soft_clip(x: f32) -> f32 {
    if x < -3.0 {
        return -1.0;
    }
    if x > 3.0 {
        return 1.0;
    }
    return soft_limit(x);
}

#[inline]
pub fn cross_fade(a: f32, b: f32, fade: f32) -> f32 {
    return a + (b - a) * fade;
}

#[inline]
pub fn interpolate_table(table: &[f32], index: f32, size: f32) -> f32 {
    let index = index * size;
    let index_integral = index as usize;
    let index_fractional = index - index_integral as f32;
    let a = table[index_integral];
    let b = table[index_integral + 1];
    return a + (b - a) * index_fractional;
}

#[inline]
pub fn clip16(x: i32) -> i16 {
    if x < -32768 {
        return -32768;
    }
    if x > 32767 {
        return 32767;
    }
    return x as i16;
}
