pub struct Point {
    pub x: f64,
    pub y: f64,
}

fn norm2(p: &Point) -> f64 {
    p.x * p.x + p.y * p.y
}

fn dist(a: &Point, b: &Point) -> f64 {
    let d = Point { x: a.x - b.x, y: a.y - b.y };
    norm2(&d).sqrt()
}

fn is_even(n: u64) -> bool {
    if n == 0 { true } else { is_odd(n - 1) }
}

fn is_odd(n: u64) -> bool {
    if n == 0 { false } else { is_even(n - 1) }
}
