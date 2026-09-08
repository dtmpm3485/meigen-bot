#[derive(Debug, Default)]
pub struct Features {
    pub contrast: bool,
    pub assertion: bool,
    pub comparison: bool,
    pub parallel: bool,
    pub reversal: bool,
    pub explanation: bool,
    pub abstract_theme: usize,
    pub mundane_theme: usize,
    pub risky: usize,
    pub theme_count: usize,
}
pub fn clamp(n: usize) -> u8 {
    n.min(100) as u8
}
pub fn scores(f: &Features) -> [u8; 5] {
    let b = |x: bool, n| if x { n } else { 0 };
    let mix = (f.abstract_theme > 0 && f.mundane_theme > 0)
        || (f.mundane_theme >= 2 && f.comparison && f.assertion);
    let structure = f.contrast || f.comparison || f.parallel || f.reversal || f.explanation;
    let score = 20
        + b(f.contrast, 16)
        + b(f.assertion, 10)
        + b(f.comparison, 12)
        + b(f.parallel, 22)
        + b(f.reversal, 22)
        + b(f.explanation, 15)
        + b(mix, 12)
        + f.theme_count.min(3) * 4;
    // Theme words alone must never reach the candidate band.
    let score = if structure { score } else { score.min(55) };
    let score = if score > 85 {
        85 + (score - 85) / 2
    } else {
        score
    };
    [
        clamp(score),
        clamp(15 + f.abstract_theme.min(4) * 15 + b(f.reversal, 20) + b(f.contrast, 10)),
        clamp(
            10 + b(f.contrast, 15)
                + b(f.reversal, 25)
                + b(f.parallel, 15)
                + b(mix, 20)
                + f.mundane_theme.min(3) * 5,
        ),
        clamp(20 + b(f.assertion, 25) + b(f.explanation, 25) + b(f.comparison, 15)),
        clamp(5 + f.risky.min(4) * 18 + b(f.mundane_theme >= 2, 20) + b(f.reversal, 10)),
    ]
}
