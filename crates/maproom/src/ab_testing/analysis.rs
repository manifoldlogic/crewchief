//! Statistical analysis tools for A/B testing
//!
//! Provides chi-square tests, t-tests, confidence intervals, and sample size
//! calculations for validating search quality improvements.

use serde::{Deserialize, Serialize};

/// Statistical test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalTestResult {
    /// Test statistic value
    pub statistic: f64,
    /// P-value (probability of observing this result under null hypothesis)
    pub p_value: f64,
    /// Whether the result is statistically significant (p < threshold)
    pub is_significant: bool,
    /// Significance threshold used (typically 0.05)
    pub significance_threshold: f64,
    /// Test description
    pub test_name: String,
}

/// Confidence interval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    /// Point estimate (mean, proportion, etc.)
    pub estimate: f64,
    /// Lower bound of confidence interval
    pub lower_bound: f64,
    /// Upper bound of confidence interval
    pub upper_bound: f64,
    /// Confidence level (e.g., 0.95 for 95%)
    pub confidence_level: f64,
}

/// Statistical analyzer for A/B test metrics
pub struct StatisticalAnalyzer {
    /// Default significance threshold (p-value threshold)
    pub significance_threshold: f64,
}

impl StatisticalAnalyzer {
    /// Create new analyzer with default significance threshold (0.05)
    pub fn new() -> Self {
        Self {
            significance_threshold: 0.05,
        }
    }

    /// Create analyzer with custom significance threshold
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            significance_threshold: threshold,
        }
    }

    /// Perform chi-square test for categorical data (e.g., click-through rates)
    ///
    /// Tests if there's a significant difference in proportions between two groups.
    ///
    /// # Arguments
    /// * `old_successes` - Number of successes in old implementation (e.g., clicks)
    /// * `old_total` - Total observations in old implementation
    /// * `new_successes` - Number of successes in new implementation
    /// * `new_total` - Total observations in new implementation
    ///
    /// # Returns
    /// Statistical test result with chi-square statistic and p-value
    pub fn chi_square_test(
        &self,
        old_successes: usize,
        old_total: usize,
        new_successes: usize,
        new_total: usize,
    ) -> anyhow::Result<StatisticalTestResult> {
        if old_total == 0 || new_total == 0 {
            return Err(anyhow::anyhow!("Sample sizes must be greater than 0"));
        }

        let old_failures = old_total - old_successes;
        let new_failures = new_total - new_successes;

        // 2x2 contingency table:
        //           Success   Failure
        // Old:      a         b
        // New:      c         d

        let a = old_successes as f64;
        let b = old_failures as f64;
        let c = new_successes as f64;
        let d = new_failures as f64;

        let n = a + b + c + d;

        // Chi-square statistic: χ² = n(ad - bc)² / ((a+b)(c+d)(a+c)(b+d))
        let numerator = n * (a * d - b * c).powi(2);
        let denominator = (a + b) * (c + d) * (a + c) * (b + d);

        if denominator == 0.0 {
            return Err(anyhow::anyhow!("Invalid contingency table"));
        }

        let chi_square = numerator / denominator;

        // Calculate p-value using chi-square distribution with 1 degree of freedom
        let p_value = self.chi_square_p_value(chi_square, 1);

        Ok(StatisticalTestResult {
            statistic: chi_square,
            p_value,
            is_significant: p_value < self.significance_threshold,
            significance_threshold: self.significance_threshold,
            test_name: "Chi-square test".to_string(),
        })
    }

    /// Perform two-sample t-test for continuous metrics (e.g., NDCG, latency)
    ///
    /// Tests if there's a significant difference in means between two groups.
    ///
    /// # Arguments
    /// * `old_values` - Sample values from old implementation
    /// * `new_values` - Sample values from new implementation
    ///
    /// # Returns
    /// Statistical test result with t-statistic and p-value
    pub fn t_test(&self, old_values: &[f64], new_values: &[f64]) -> anyhow::Result<StatisticalTestResult> {
        if old_values.is_empty() || new_values.is_empty() {
            return Err(anyhow::anyhow!("Sample sizes must be greater than 0"));
        }

        let n1 = old_values.len() as f64;
        let n2 = new_values.len() as f64;

        // Calculate means
        let mean1 = old_values.iter().sum::<f64>() / n1;
        let mean2 = new_values.iter().sum::<f64>() / n2;

        // Calculate variances
        let var1 = old_values
            .iter()
            .map(|x| (x - mean1).powi(2))
            .sum::<f64>()
            / (n1 - 1.0);
        let var2 = new_values
            .iter()
            .map(|x| (x - mean2).powi(2))
            .sum::<f64>()
            / (n2 - 1.0);

        // Pooled standard error
        let se = ((var1 / n1) + (var2 / n2)).sqrt();

        if se == 0.0 {
            return Err(anyhow::anyhow!("Standard error is zero"));
        }

        // T-statistic
        let t = (mean2 - mean1) / se;

        // Degrees of freedom (Welch-Satterthwaite approximation)
        let df = ((var1 / n1 + var2 / n2).powi(2))
            / ((var1 / n1).powi(2) / (n1 - 1.0) + (var2 / n2).powi(2) / (n2 - 1.0));

        // Calculate p-value (two-tailed)
        let p_value = self.t_distribution_p_value(t.abs(), df);

        Ok(StatisticalTestResult {
            statistic: t,
            p_value,
            is_significant: p_value < self.significance_threshold,
            significance_threshold: self.significance_threshold,
            test_name: "Two-sample t-test".to_string(),
        })
    }

    /// Calculate confidence interval for a proportion
    ///
    /// # Arguments
    /// * `successes` - Number of successes
    /// * `total` - Total observations
    /// * `confidence_level` - Desired confidence level (e.g., 0.95 for 95%)
    ///
    /// # Returns
    /// Confidence interval for the proportion
    pub fn proportion_confidence_interval(
        &self,
        successes: usize,
        total: usize,
        confidence_level: f64,
    ) -> anyhow::Result<ConfidenceInterval> {
        if total == 0 {
            return Err(anyhow::anyhow!("Total must be greater than 0"));
        }

        let p = successes as f64 / total as f64;
        let n = total as f64;

        // Z-score for confidence level (using standard normal distribution)
        let z = self.z_score_for_confidence_level(confidence_level);

        // Standard error
        let se = (p * (1.0 - p) / n).sqrt();

        // Confidence interval
        let margin = z * se;

        Ok(ConfidenceInterval {
            estimate: p,
            lower_bound: (p - margin).max(0.0),
            upper_bound: (p + margin).min(1.0),
            confidence_level,
        })
    }

    /// Calculate confidence interval for a mean
    ///
    /// # Arguments
    /// * `values` - Sample values
    /// * `confidence_level` - Desired confidence level (e.g., 0.95 for 95%)
    ///
    /// # Returns
    /// Confidence interval for the mean
    pub fn mean_confidence_interval(
        &self,
        values: &[f64],
        confidence_level: f64,
    ) -> anyhow::Result<ConfidenceInterval> {
        if values.is_empty() {
            return Err(anyhow::anyhow!("Sample size must be greater than 0"));
        }

        let n = values.len() as f64;
        let mean = values.iter().sum::<f64>() / n;

        // Sample standard deviation
        let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let std_dev = variance.sqrt();

        // Standard error
        let se = std_dev / n.sqrt();

        // T-score for confidence level
        let df = n - 1.0;
        let t = self.t_score_for_confidence_level(confidence_level, df);

        // Confidence interval
        let margin = t * se;

        Ok(ConfidenceInterval {
            estimate: mean,
            lower_bound: mean - margin,
            upper_bound: mean + margin,
            confidence_level,
        })
    }

    /// Calculate required sample size for detecting a minimum effect size
    ///
    /// # Arguments
    /// * `baseline_rate` - Baseline success rate (e.g., 0.10 for 10% CTR)
    /// * `minimum_detectable_effect` - Minimum effect to detect (e.g., 0.02 for 2% absolute increase)
    /// * `power` - Statistical power (typically 0.80 for 80%)
    /// * `significance_level` - Significance level (typically 0.05)
    ///
    /// # Returns
    /// Required sample size per group
    pub fn calculate_sample_size(
        &self,
        baseline_rate: f64,
        minimum_detectable_effect: f64,
        power: f64,
        significance_level: f64,
    ) -> anyhow::Result<usize> {
        if baseline_rate <= 0.0 || baseline_rate >= 1.0 {
            return Err(anyhow::anyhow!("Baseline rate must be between 0 and 1"));
        }

        // Z-scores for alpha/2 and beta
        let z_alpha = self.z_score_for_confidence_level(1.0 - significance_level);
        let z_beta = self.z_score_for_confidence_level(power);

        let p1 = baseline_rate;
        let p2 = baseline_rate + minimum_detectable_effect;

        // Pooled proportion
        let p_pooled = (p1 + p2) / 2.0;

        // Sample size formula for two proportions
        let numerator =
            (z_alpha * (2.0 * p_pooled * (1.0 - p_pooled)).sqrt()
                + z_beta * (p1 * (1.0 - p1) + p2 * (1.0 - p2)).sqrt())
            .powi(2);
        let denominator = (p2 - p1).powi(2);

        let n = (numerator / denominator).ceil() as usize;

        Ok(n)
    }

    // Helper functions for statistical distributions

    /// Approximate p-value for chi-square distribution
    fn chi_square_p_value(&self, chi_square: f64, df: usize) -> f64 {
        // Simplified approximation using incomplete gamma function
        // For production, use a statistical library like statrs
        if chi_square < 0.0 {
            return 1.0;
        }

        // Very rough approximation for df=1
        if df == 1 {
            if chi_square > 10.83 {
                return 0.001;
            } else if chi_square > 6.63 {
                return 0.01;
            } else if chi_square > 3.84 {
                return 0.05;
            } else if chi_square > 2.71 {
                return 0.10;
            } else {
                return 0.50;
            }
        }

        0.05 // Default conservative estimate
    }

    /// Approximate p-value for t-distribution (two-tailed)
    fn t_distribution_p_value(&self, t: f64, df: f64) -> f64 {
        // Simplified approximation
        // For production, use a statistical library like statrs
        let abs_t = t.abs();

        if df > 30.0 {
            // Use normal approximation for large df
            return 2.0 * self.standard_normal_cdf(-abs_t);
        }

        // Very rough critical values for common df
        if abs_t > 2.58 {
            0.01
        } else if abs_t > 1.96 {
            0.05
        } else if abs_t > 1.64 {
            0.10
        } else {
            0.50
        }
    }

    /// Standard normal CDF approximation
    fn standard_normal_cdf(&self, x: f64) -> f64 {
        // Simple approximation using error function
        0.5 * (1.0 + self.erf(x / 2_f64.sqrt()))
    }

    /// Error function approximation
    fn erf(&self, x: f64) -> f64 {
        // Abramowitz and Stegun approximation
        let a1 = 0.254829592;
        let a2 = -0.284496736;
        let a3 = 1.421413741;
        let a4 = -1.453152027;
        let a5 = 1.061405429;
        let p = 0.3275911;

        let sign = if x < 0.0 { -1.0 } else { 1.0 };
        let x = x.abs();

        let t = 1.0 / (1.0 + p * x);
        let y = 1.0 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * (-x * x).exp();

        sign * y
    }

    /// Get z-score for confidence level
    fn z_score_for_confidence_level(&self, confidence_level: f64) -> f64 {
        // Common z-scores
        match confidence_level {
            x if (x - 0.90).abs() < 0.001 => 1.645,
            x if (x - 0.95).abs() < 0.001 => 1.96,
            x if (x - 0.99).abs() < 0.001 => 2.576,
            _ => 1.96, // Default to 95%
        }
    }

    /// Get t-score for confidence level
    fn t_score_for_confidence_level(&self, confidence_level: f64, df: f64) -> f64 {
        // For large df, use z-scores
        if df > 30.0 {
            return self.z_score_for_confidence_level(confidence_level);
        }

        // Rough approximations for small df and 95% confidence
        if (confidence_level - 0.95).abs() < 0.001 {
            if df < 5.0 {
                2.776
            } else if df < 10.0 {
                2.262
            } else if df < 20.0 {
                2.093
            } else {
                2.042
            }
        } else {
            self.z_score_for_confidence_level(confidence_level)
        }
    }
}

impl Default for StatisticalAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chi_square_test() {
        let analyzer = StatisticalAnalyzer::new();

        // Test with clear difference (should be significant)
        let result = analyzer
            .chi_square_test(100, 1000, 150, 1000)
            .unwrap();

        assert!(result.statistic > 0.0);
        assert!(result.p_value >= 0.0 && result.p_value <= 1.0);
    }

    #[test]
    fn test_t_test() {
        let analyzer = StatisticalAnalyzer::new();

        let old_values = vec![0.75, 0.78, 0.76, 0.74, 0.77];
        let new_values = vec![0.82, 0.85, 0.83, 0.84, 0.86];

        let result = analyzer.t_test(&old_values, &new_values).unwrap();

        assert!(result.statistic != 0.0);
        assert!(result.p_value >= 0.0 && result.p_value <= 1.0);
    }

    #[test]
    fn test_proportion_confidence_interval() {
        let analyzer = StatisticalAnalyzer::new();

        let ci = analyzer
            .proportion_confidence_interval(100, 1000, 0.95)
            .unwrap();

        assert_eq!(ci.estimate, 0.1);
        assert!(ci.lower_bound < ci.estimate);
        assert!(ci.upper_bound > ci.estimate);
        assert!(ci.lower_bound >= 0.0);
        assert!(ci.upper_bound <= 1.0);
    }

    #[test]
    fn test_mean_confidence_interval() {
        let analyzer = StatisticalAnalyzer::new();

        let values = vec![0.75, 0.78, 0.76, 0.74, 0.77];
        let ci = analyzer.mean_confidence_interval(&values, 0.95).unwrap();

        assert!(ci.lower_bound < ci.estimate);
        assert!(ci.upper_bound > ci.estimate);
    }

    #[test]
    fn test_sample_size_calculation() {
        let analyzer = StatisticalAnalyzer::new();

        let n = analyzer
            .calculate_sample_size(0.10, 0.02, 0.80, 0.05)
            .unwrap();

        assert!(n > 0);
        // Typical sample sizes for this scenario should be in thousands
        assert!(n > 100);
    }

    #[test]
    fn test_empty_samples() {
        let analyzer = StatisticalAnalyzer::new();

        assert!(analyzer.t_test(&[], &[1.0]).is_err());
        assert!(analyzer.t_test(&[1.0], &[]).is_err());
        assert!(analyzer.mean_confidence_interval(&[], 0.95).is_err());
    }
}
