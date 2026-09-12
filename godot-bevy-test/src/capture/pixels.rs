use std::collections::BTreeMap;

pub(super) fn summarize(
    pixels: impl IntoIterator<Item = [u8; 3]>,
    background: [u8; 3],
    tolerance: u8,
) -> (f64, [u8; 3]) {
    let mut colours = BTreeMap::<[u8; 3], usize>::new();
    let mut total = 0;
    let mut non_blank = 0;
    for rgb in pixels {
        total += 1;
        non_blank += usize::from(
            rgb.iter()
                .zip(background)
                .any(|(channel, bg)| channel.abs_diff(bg) > tolerance),
        );
        *colours.entry(rgb).or_default() += 1;
    }
    let dominant = colours
        .into_iter()
        .max_by(|(a, count_a), (b, count_b)| count_a.cmp(count_b).then_with(|| b.cmp(a)))
        .map(|(rgb, _)| rgb)
        .unwrap_or_default();
    (non_blank as f64 / total.max(1) as f64, dominant)
}
