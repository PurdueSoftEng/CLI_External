pub mod github;
pub mod npm;

use statrs::distribution::{Continuous, Normal};

pub trait Metrics {
    // calculate metrics of module
    // returns value in range [0, 1]

    // ramp up time for engineers to learn module
    fn ramp_up_time(&self) -> f64;

    // correctness of module
    fn correctness(&self) -> f64;

    // whether there are enough maintainers for module
    fn bus_factor(&self) -> f64;

    // responsiveness of module maintainers fixing bugs
    fn responsiveness(&self) -> f64;

    // whether module is compatible with LGPLv2.1
    fn compatibility(&self) -> f64;

    // New pinning_practice metric for part 2
    fn pinning_practice(&self) -> f64;

    // formulas for calculating metrics
    fn calc_ramp_up_time(lines: u32) -> f64
    where
        Self: Sized,
    {
        let mut x = lines as f64;
        x = x / 150.0 * 0.7;
        let normal = Normal::new(0.0, 1.0).unwrap();

        normal.pdf(x) * x.sqrt() / 0.2613
    }

    fn calc_correctness(all: u32, closed: u32) -> f64
    where
        Self: Sized,
    {
        if all == 0 || all < closed {
            0.0
        } else {
            closed as f64 / all as f64
        }
    }

    fn calc_compatibility(license: &str) -> f64
    where
        Self: Sized,
    {
        let acceptable = [
            "LGPL-2.1-only",
            "LGPL-2.1",
            "LGPL-2.1-or-later",
            "LGPL-3.0-only",
            "LGPL-3.0",
            "BSD-3-Clause",
            "MIT",
            "X11",
            "CC0-1.0",
            "Unlicense",
        ];
        if acceptable.contains(&license) {
            1.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMetrics;
    impl Metrics for TestMetrics {
        fn ramp_up_time(&self) -> f64 {
            0.0
        }
        fn correctness(&self) -> f64 {
            0.0
        }
        fn bus_factor(&self) -> f64 {
            0.0
        }
        fn responsiveness(&self) -> f64 {
            0.0
        }
        fn compatibility(&self) -> f64 {
            0.0
        }
        fn pinning_practice(&self) -> f64 {
            0.0
        }
    }

    #[test]
    fn ramp_up_time_edge_cases() {
        assert!(TestMetrics::calc_ramp_up_time(0) == 0.0);
        assert!(TestMetrics::calc_ramp_up_time(u32::MAX) == 0.0);
        assert!(TestMetrics::calc_ramp_up_time(1000) <= 0.1);
    }

    #[test]
    fn ramp_up_time_max() {
        assert!(TestMetrics::calc_ramp_up_time(150) >= 0.99);
    }

    #[test]
    fn correctness_edge_cases() {
        assert!(TestMetrics::calc_correctness(0, 0) == 0.0);
        assert!(TestMetrics::calc_correctness(100, 0) == 0.0);
        assert!(TestMetrics::calc_correctness(100, 100) == 1.0);
        assert!(TestMetrics::calc_correctness(0, 100) == 0.0);
    }

    #[test]
    fn correctness_normal_cases() {
        assert!(TestMetrics::calc_correctness(2000, 1900) == 1900.0 / 2000.0);
        assert!(TestMetrics::calc_correctness(2000, 100) == 100.0 / 2000.0);
    }

    #[test]
    fn compatibility_tests() {
        assert!(TestMetrics::calc_compatibility("MIT") == 1.0);
        assert!(TestMetrics::calc_compatibility("LGPL-2.1") == 1.0);
        assert!(TestMetrics::calc_compatibility("notMIT") == 0.0);
        assert!(TestMetrics::calc_compatibility("") == 0.0);
    }
}
