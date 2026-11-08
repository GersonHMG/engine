// evaluation/stats.rs — Is the candidate's win rate different from the baseline's?
//
// Both sides play the same episodes (same scenario, same jittered start), so the
// comparison is paired:
//   * per side: the win rate with its Wilson 95% interval;
//   * the difference candidate − baseline: a bootstrap 95% interval (episodes resampled
//     within each scenario, seeded, so the numbers are reproducible);
//   * the test: exact McNemar on the discordant pairs (won only by the baseline vs won only
//     by the candidate), two-sided; over several scenarios the p values are Holm-adjusted.
//   * time to goal: the mean difference over the episodes both sides scored, with a
//     bootstrap interval.
// A verdict is "better" / "worse" only when the overall McNemar test rejects at ALPHA.

use std::collections::BTreeMap;

use super::{Episode, Evaluation};

/// Significance level of the verdict.
pub const ALPHA: f64 = 0.05;
/// Bootstrap resamples.
pub const BOOTSTRAP: usize = 2000;
const Z95: f64 = 1.959_963_984_540_054;

/// A value with its 95% interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Estimate {
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The candidate wins significantly more often.
    Better,
    /// The candidate wins significantly less often.
    Worse,
    /// No significant difference at ALPHA.
    Inconclusive,
}

/// One scenario (or all of them, `name` = "All").
#[derive(Debug, Clone)]
pub struct Comparison {
    pub name: String,
    /// Paired episodes (played by both sides).
    pub n: usize,
    pub baseline: Estimate,
    pub candidate: Estimate,
    /// Candidate − baseline win rate.
    pub diff: Estimate,
    /// Episodes won only by the baseline / only by the candidate.
    pub only_baseline: usize,
    pub only_candidate: usize,
    /// Exact McNemar, two-sided.
    pub p: f64,
    /// Holm-adjusted over the scenarios (equals `p` for "All").
    pub p_adjusted: f64,
    /// Candidate − baseline mean time to goal, s, over episodes both scored (None if none).
    pub time_diff: Option<Estimate>,
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub scenarios: Vec<Comparison>,
    pub overall: Comparison,
    pub verdict: Verdict,
    /// Result label -> episodes, per side.
    pub outcomes_baseline: BTreeMap<String, usize>,
    pub outcomes_candidate: BTreeMap<String, usize>,
    /// Times to goal, s, per side (every goal, not only paired ones).
    pub goal_times_baseline: Vec<f64>,
    pub goal_times_candidate: Vec<f64>,
    /// Episodes only one side played (a failed run): left out of the comparison.
    pub unpaired: usize,
}

/// Wilson score interval of `k` successes in `n` trials.
pub fn wilson(k: usize, n: usize) -> Estimate {
    if n == 0 {
        return Estimate { value: f64::NAN, lo: 0.0, hi: 1.0 };
    }
    let (k, n) = (k as f64, n as f64);
    let p = k / n;
    let z2 = Z95 * Z95;
    let centre = (p + z2 / (2.0 * n)) / (1.0 + z2 / n);
    let half = Z95 * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt() / (1.0 + z2 / n);
    Estimate { value: p, lo: (centre - half).max(0.0), hi: (centre + half).min(1.0) }
}

fn ln_gamma(x: f64) -> f64 {
    // Lanczos approximation (g = 7, n = 9), accurate to ~1e-15 for x > 0.
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        let pi = std::f64::consts::PI;
        return (pi / (pi * x).sin()).ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + 7.5;
    for (i, c) in C.iter().enumerate().skip(1) {
        a += c / (x + i as f64);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// P(X <= k) for X ~ Binomial(n, 1/2).
fn binomial_half_cdf(k: usize, n: usize) -> f64 {
    let ln_n = ln_gamma(n as f64 + 1.0);
    let ln_half_n = n as f64 * 0.5f64.ln();
    (0..=k.min(n))
        .map(|i| (ln_n - ln_gamma(i as f64 + 1.0) - ln_gamma((n - i) as f64 + 1.0) + ln_half_n).exp())
        .sum::<f64>()
        .min(1.0)
}

/// Exact two-sided McNemar test: `b` pairs won only by one side, `c` only by the other.
pub fn mcnemar(b: usize, c: usize) -> f64 {
    let n = b + c;
    if n == 0 {
        return 1.0;
    }
    (2.0 * binomial_half_cdf(b.min(c), n)).min(1.0)
}

/// Holm–Bonferroni adjusted p values, in the input order.
pub fn holm(p: &[f64]) -> Vec<f64> {
    let m = p.len();
    let mut order: Vec<usize> = (0..m).collect();
    order.sort_by(|&a, &b| p[a].total_cmp(&p[b]));
    let mut adjusted = vec![0.0; m];
    let mut running: f64 = 0.0;
    for (rank, &i) in order.iter().enumerate() {
        let value = ((m - rank) as f64 * p[i]).min(1.0);
        running = running.max(value);
        adjusted[i] = running;
    }
    adjusted
}

/// SplitMix64: a small seeded generator for the bootstrap (reproducible numbers).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// The `q` quantile (0..1) of sorted values, linear interpolation.
pub fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let pos = q.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let (i, frac) = (pos.floor() as usize, pos.fract());
    if i + 1 < sorted.len() { sorted[i] * (1.0 - frac) + sorted[i + 1] * frac } else { sorted[i] }
}

/// Mean of `values` per stratum, resampled with replacement within each stratum: the
/// percentile 95% interval of the overall mean (weighted by stratum size).
fn bootstrap_mean(strata: &[Vec<f64>], seed: u64) -> Option<Estimate> {
    let total: usize = strata.iter().map(Vec::len).sum();
    if total == 0 {
        return None;
    }
    let mean = strata.iter().flatten().sum::<f64>() / total as f64;
    let mut rng = Rng(seed);
    let mut means = Vec::with_capacity(BOOTSTRAP);
    for _ in 0..BOOTSTRAP {
        let mut sum = 0.0;
        for stratum in strata.iter().filter(|s| !s.is_empty()) {
            for _ in 0..stratum.len() {
                sum += stratum[rng.below(stratum.len())];
            }
        }
        means.push(sum / total as f64);
    }
    means.sort_by(f64::total_cmp);
    Some(Estimate { value: mean, lo: quantile(&means, 0.025), hi: quantile(&means, 0.975) })
}

/// A scenario's paired episodes: (baseline episode, candidate episode), by rep.
type Pairs<'a> = Vec<(&'a Episode, &'a Episode)>;

fn compare(name: &str, strata: &[Pairs<'_>], seed: u64) -> Comparison {
    let pairs: Vec<&(&Episode, &Episode)> = strata.iter().flatten().collect();
    let n = pairs.len();
    let wins_b = pairs.iter().filter(|(b, _)| b.won()).count();
    let wins_c = pairs.iter().filter(|(_, c)| c.won()).count();
    let only_baseline = pairs.iter().filter(|(b, c)| b.won() && !c.won()).count();
    let only_candidate = pairs.iter().filter(|(b, c)| !b.won() && c.won()).count();

    let diffs: Vec<Vec<f64>> = strata
        .iter()
        .map(|s| s.iter().map(|(b, c)| c.won() as u8 as f64 - b.won() as u8 as f64).collect())
        .collect();
    let diff = bootstrap_mean(&diffs, seed).unwrap_or(Estimate { value: f64::NAN, lo: f64::NAN, hi: f64::NAN });

    let times: Vec<Vec<f64>> = strata
        .iter()
        .map(|s| s.iter().filter(|(b, c)| b.won() && c.won()).map(|(b, c)| c.time - b.time).collect())
        .collect();
    let time_diff = bootstrap_mean(&times, seed ^ 0x5EED);

    let p = mcnemar(only_baseline, only_candidate);
    Comparison {
        name: name.to_string(),
        n,
        baseline: wilson(wins_b, n),
        candidate: wilson(wins_c, n),
        diff,
        only_baseline,
        only_candidate,
        p,
        p_adjusted: p,
        time_diff,
    }
}

fn seed_of(name: &str) -> u64 {
    // FNV-1a: a fixed seed per scenario name.
    name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// The paired comparison of an evaluation.
pub fn summarize(eval: &Evaluation) -> Summary {
    let index = |episodes: &[Episode]| -> BTreeMap<(String, u32), Episode> {
        episodes.iter().map(|e| ((e.scenario.clone(), e.rep), e.clone())).collect()
    };
    let base = index(&eval.baseline.episodes);
    let cand = index(&eval.candidate.episodes);

    let mut by_scenario: BTreeMap<&str, Pairs<'_>> = BTreeMap::new();
    let mut unpaired = 0;
    for (key, b) in &base {
        match cand.get(key) {
            Some(c) => by_scenario.entry(key.0.as_str()).or_default().push((b, c)),
            None => unpaired += 1,
        }
    }
    unpaired += cand.keys().filter(|k| !base.contains_key(*k)).count();

    // Scenario order: as in the settings, then any others.
    let mut names: Vec<&str> = eval.settings.scenarios.iter().map(String::as_str).filter(|n| by_scenario.contains_key(n)).collect();
    for name in by_scenario.keys() {
        if !names.contains(name) {
            names.push(name);
        }
    }

    let mut scenarios: Vec<Comparison> = names
        .iter()
        .map(|name| compare(name, std::slice::from_ref(&by_scenario[name]), seed_of(name)))
        .collect();
    let adjusted = holm(&scenarios.iter().map(|c| c.p).collect::<Vec<_>>());
    for (c, p) in scenarios.iter_mut().zip(adjusted) {
        c.p_adjusted = p;
    }

    let strata: Vec<Pairs<'_>> = names.iter().map(|n| by_scenario[n].clone()).collect();
    let overall = compare("All", &strata, seed_of("All"));
    let verdict = if overall.n == 0 || overall.p >= ALPHA {
        Verdict::Inconclusive
    } else if overall.only_candidate > overall.only_baseline {
        Verdict::Better
    } else {
        Verdict::Worse
    };

    let outcomes = |episodes: &[Episode]| {
        let mut counts = BTreeMap::new();
        for e in episodes {
            *counts.entry(e.result.clone()).or_insert(0) += 1;
        }
        counts
    };
    let goal_times = |episodes: &[Episode]| episodes.iter().filter(|e| e.won()).map(|e| e.time).collect::<Vec<_>>();

    Summary {
        scenarios,
        overall,
        verdict,
        outcomes_baseline: outcomes(&eval.baseline.episodes),
        outcomes_candidate: outcomes(&eval.candidate.episodes),
        goal_times_baseline: goal_times(&eval.baseline.episodes),
        goal_times_candidate: goal_times(&eval.candidate.episodes),
        unpaired,
    }
}

/// Fraction of a side's episodes that scored by time t, for t on `grid` (a cumulative goal
/// curve: how fast the attack scores, and how often in the end).
pub fn goal_curve(goal_times: &[f64], episodes: usize, grid: &[f64]) -> Vec<(f64, f64)> {
    let mut sorted = goal_times.to_vec();
    sorted.sort_by(f64::total_cmp);
    grid.iter()
        .map(|&t| {
            let scored = sorted.partition_point(|&g| g <= t);
            (t, if episodes == 0 { 0.0 } else { scored as f64 / episodes as f64 })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluation::{EvalSettings, SideResults};

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn wilson_matches_the_textbook_values() {
        // 8 of 10: 0.4902 – 0.9433 (Wilson, z = 1.96).
        let e = wilson(8, 10);
        assert!(close(e.value, 0.8, 1e-12));
        assert!(close(e.lo, 0.4902, 1e-3) && close(e.hi, 0.9433, 1e-3), "{e:?}");
        let zero = wilson(0, 20);
        assert_eq!(zero.lo, 0.0);
        assert!(close(zero.hi, 0.1611, 1e-3), "{zero:?}");
    }

    #[test]
    fn mcnemar_exact_p_values() {
        // b = 1, c = 9: 2 · P(X <= 1), X ~ Bin(10, 1/2) = 2 · 11/1024.
        assert!(close(mcnemar(1, 9), 22.0 / 1024.0, 1e-12));
        assert!(close(mcnemar(9, 1), 22.0 / 1024.0, 1e-12));
        assert_eq!(mcnemar(0, 0), 1.0);
        assert_eq!(mcnemar(5, 5), 1.0);
        // 0 vs 6: 2 / 64.
        assert!(close(mcnemar(0, 6), 2.0 / 64.0, 1e-12));
    }

    #[test]
    fn holm_adjusts_in_step_down_order() {
        let p = [0.01, 0.04, 0.03, 0.2];
        let a = holm(&p);
        // sorted 0.01·4 = 0.04, 0.03·3 = 0.09, 0.04·2 = 0.08 -> 0.09 (monotone), 0.2·1 = 0.2
        assert!(close(a[0], 0.04, 1e-12));
        assert!(close(a[2], 0.09, 1e-12));
        assert!(close(a[1], 0.09, 1e-12));
        assert!(close(a[3], 0.2, 1e-12));
    }

    #[test]
    fn quantile_interpolates() {
        let v = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(quantile(&v, 0.0), 1.0);
        assert_eq!(quantile(&v, 1.0), 4.0);
        assert!(close(quantile(&v, 0.5), 2.5, 1e-12));
    }

    fn episodes(scenario: &str, wins: &[bool]) -> Vec<Episode> {
        wins.iter()
            .enumerate()
            .map(|(i, &w)| Episode {
                scenario: scenario.into(),
                rep: i as u32 + 1,
                result: if w { "goal".into() } else { "timeout".into() },
                time: if w { 3.0 } else { 15.0 },
                kicks_blue: 1,
                kicks_yellow: 0,
            })
            .collect()
    }

    fn evaluation(base: Vec<Episode>, cand: Vec<Episode>) -> Evaluation {
        Evaluation {
            settings: EvalSettings { scenarios: vec!["a".into(), "b".into()], ..EvalSettings::default() },
            baseline: SideResults { label: "base".into(), episodes: base },
            candidate: SideResults { label: "cand".into(), episodes: cand },
            created: 0,
        }
    }

    #[test]
    fn a_clearly_better_candidate_is_better() {
        let mut base = episodes("a", &[false; 20]);
        base.extend(episodes("b", &[true; 20]));
        let mut cand = episodes("a", &[true; 20]);
        cand.extend(episodes("b", &[true; 20]));
        let s = summarize(&evaluation(base, cand));
        assert_eq!(s.verdict, Verdict::Better);
        assert_eq!(s.overall.n, 40);
        assert!(close(s.overall.diff.value, 0.5, 1e-12));
        assert!(s.overall.diff.lo > 0.0);
        assert_eq!(s.scenarios[0].name, "a");
        assert!(close(s.scenarios[0].diff.value, 1.0, 1e-12));
        assert_eq!(s.scenarios[1].p, 1.0);
    }

    #[test]
    fn identical_sides_are_inconclusive_and_unpaired_episodes_are_counted() {
        let base = episodes("a", &[true, false, true, false]);
        let mut cand = base.clone();
        cand.push(Episode { rep: 99, ..cand[0].clone() });
        let s = summarize(&evaluation(base, cand));
        assert_eq!(s.verdict, Verdict::Inconclusive);
        assert_eq!(s.unpaired, 1);
        assert_eq!(s.overall.diff.value, 0.0);
        assert_eq!(s.overall.time_diff.unwrap().value, 0.0);
    }

    #[test]
    fn goal_curve_counts_goals_by_time() {
        let curve = goal_curve(&[1.0, 2.0, 2.0], 4, &[0.5, 1.0, 2.0, 3.0]);
        assert_eq!(curve, vec![(0.5, 0.0), (1.0, 0.25), (2.0, 0.75), (3.0, 0.75)]);
    }
}
