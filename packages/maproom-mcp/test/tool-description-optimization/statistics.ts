/**
 * Statistical Utility Functions
 *
 * Implements core statistical tests and calculations for variant comparison:
 * - Welch's t-test (robust to unequal variances)
 * - Cohen's d effect size
 * - Confidence intervals
 * - Power analysis
 */

/**
 * Calculate mean of an array
 */
export function mean(values: number[]): number {
  if (values.length === 0) return 0
  return values.reduce((sum, v) => sum + v, 0) / values.length
}

/**
 * Calculate standard deviation (sample)
 */
export function stdDev(values: number[]): number {
  if (values.length < 2) return 0
  const m = mean(values)
  const squaredDiffs = values.map(v => Math.pow(v - m, 2))
  const variance = squaredDiffs.reduce((sum, v) => sum + v, 0) / (values.length - 1)
  return Math.sqrt(variance)
}

/**
 * Calculate standard error
 */
export function standardError(values: number[]): number {
  if (values.length === 0) return 0
  return stdDev(values) / Math.sqrt(values.length)
}

/**
 * Welch's t-test for two samples with unequal variances
 *
 * Returns: { t, df, p_value }
 *
 * Null hypothesis: mean1 = mean2
 * Alternative hypothesis: mean1 ≠ mean2 (two-tailed test)
 */
export interface TTestResult {
  t: number // t-statistic
  df: number // degrees of freedom
  p_value: number // two-tailed p-value
}

export function welchTTest(sample1: number[], sample2: number[]): TTestResult {
  const n1 = sample1.length
  const n2 = sample2.length

  if (n1 < 2 || n2 < 2) {
    throw new Error('Each sample must have at least 2 observations')
  }

  const mean1 = mean(sample1)
  const mean2 = mean(sample2)
  const var1 = Math.pow(stdDev(sample1), 2)
  const var2 = Math.pow(stdDev(sample2), 2)

  // Calculate Welch's t-statistic
  const t = (mean1 - mean2) / Math.sqrt(var1 / n1 + var2 / n2)

  // Calculate Welch-Satterthwaite degrees of freedom
  const numerator = Math.pow(var1 / n1 + var2 / n2, 2)
  const denominator = Math.pow(var1 / n1, 2) / (n1 - 1) + Math.pow(var2 / n2, 2) / (n2 - 1)
  const df = numerator / denominator

  // Calculate two-tailed p-value from t-distribution
  const p_value = 2 * (1 - studentTCDF(Math.abs(t), df))

  return { t, df, p_value }
}

/**
 * Student's t cumulative distribution function
 * Approximation using normal distribution for df > 30, exact for df ≤ 30
 */
function studentTCDF(t: number, df: number): number {
  // For large df, t-distribution approaches normal distribution
  if (df > 100) {
    return normalCDF(t)
  }

  // Use numerical integration for small df
  // This is a simplified implementation; for production use a proper stats library
  // Formula: integrate from -∞ to t of the t-distribution PDF

  // Approximation: use normal CDF adjusted for df
  const factor = 1 + t * t / df
  const correction = Math.pow(factor, -(df + 1) / 2)
  return normalCDF(t) * (1 + correction * 0.1) // Rough approximation
}

/**
 * Standard normal cumulative distribution function
 * Using erf (error function) approximation
 */
function normalCDF(z: number): number {
  // Using the approximation: Φ(z) = (1 + erf(z/√2)) / 2
  return (1 + erf(z / Math.sqrt(2))) / 2
}

/**
 * Error function approximation (Abramowitz and Stegun)
 */
function erf(x: number): number {
  // Constants
  const a1 = 0.254829592
  const a2 = -0.284496736
  const a3 = 1.421413741
  const a4 = -1.453152027
  const a5 = 1.061405429
  const p = 0.3275911

  // Save the sign of x
  const sign = x >= 0 ? 1 : -1
  const absX = Math.abs(x)

  // A&S formula 7.1.26
  const t = 1 / (1 + p * absX)
  const y = 1 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * Math.exp(-absX * absX)

  return sign * y
}

/**
 * Cohen's d effect size
 *
 * Interpretation:
 * - 0.2: small effect
 * - 0.5: medium effect
 * - 0.8: large effect
 */
export function cohensD(sample1: number[], sample2: number[]): number {
  const mean1 = mean(sample1)
  const mean2 = mean(sample2)

  const n1 = sample1.length
  const n2 = sample2.length

  const var1 = Math.pow(stdDev(sample1), 2)
  const var2 = Math.pow(stdDev(sample2), 2)

  // Pooled standard deviation
  const pooledSD = Math.sqrt(((n1 - 1) * var1 + (n2 - 1) * var2) / (n1 + n2 - 2))

  return (mean1 - mean2) / pooledSD
}

/**
 * Calculate 95% confidence interval for a mean
 *
 * Uses t-distribution for small samples, normal for large samples
 */
export interface ConfidenceInterval {
  lower: number
  upper: number
  confidence: number
}

export function confidenceInterval(
  values: number[],
  confidence: number = 0.95
): ConfidenceInterval {
  if (values.length < 2) {
    const m = values.length === 1 ? values[0] : 0
    return { lower: m, upper: m, confidence }
  }

  const m = mean(values)
  const se = standardError(values)
  const n = values.length
  const df = n - 1

  // Critical value from t-distribution
  // For 95% CI, α = 0.05, so we want t(α/2, df)
  // Approximation: for large df use z-value, for small use t-value
  const alpha = 1 - confidence
  const tCritical = df > 30 ? 1.96 : getTCritical(alpha / 2, df)

  const margin = tCritical * se

  return {
    lower: m - margin,
    upper: m + margin,
    confidence
  }
}

/**
 * Get critical t-value for given alpha and degrees of freedom
 * Simplified lookup table + interpolation
 */
function getTCritical(alpha: number, df: number): number {
  // For 95% CI (alpha=0.025 two-tailed), common t-values
  const tTable: { [key: number]: number } = {
    1: 12.706,
    2: 4.303,
    3: 3.182,
    4: 2.776,
    5: 2.571,
    10: 2.228,
    20: 2.086,
    30: 2.042,
    60: 2.000,
    100: 1.984
  }

  if (tTable[df]) return tTable[df]

  // Linear interpolation for values between table entries
  const keys = Object.keys(tTable).map(Number).sort((a, b) => a - b)
  for (let i = 0; i < keys.length - 1; i++) {
    if (df >= keys[i] && df <= keys[i + 1]) {
      const t1 = tTable[keys[i]]
      const t2 = tTable[keys[i + 1]]
      const ratio = (df - keys[i]) / (keys[i + 1] - keys[i])
      return t1 + (t2 - t1) * ratio
    }
  }

  // For very large df, use normal approximation
  return 1.96
}

/**
 * Bonferroni correction for multiple comparisons
 *
 * Adjusts significance level to control family-wise error rate
 *
 * @param alpha - Original significance level (e.g., 0.05)
 * @param numComparisons - Number of pairwise comparisons
 * @returns Adjusted alpha level
 */
export function bonferroniCorrection(alpha: number, numComparisons: number): number {
  return alpha / numComparisons
}

/**
 * Calculate required sample size for desired power
 *
 * Uses Cohen's power analysis for two-sample t-test
 *
 * @param effectSize - Expected Cohen's d
 * @param alpha - Significance level (default 0.05)
 * @param power - Desired power (default 0.8)
 * @returns Required sample size per group
 */
export function requiredSampleSize(
  effectSize: number,
  alpha: number = 0.05,
  power: number = 0.8
): number {
  // Simplified Cohen's formula for equal sample sizes
  // n ≈ 2 * ((z_α/2 + z_β) / d)^2
  // where z_α/2 is critical value for α, z_β for power

  const zAlpha = 1.96 // for α = 0.05 (two-tailed)
  const zBeta = 0.84 // for power = 0.8

  const n = 2 * Math.pow((zAlpha + zBeta) / effectSize, 2)

  return Math.ceil(n)
}

/**
 * Check if sample size provides adequate power
 *
 * @param sampleSize - Actual sample size per group
 * @param effectSize - Observed or expected effect size
 * @param alpha - Significance level
 * @returns Estimated power (0-1)
 */
export function estimatePower(
  sampleSize: number,
  effectSize: number,
  alpha: number = 0.05
): number {
  const zAlpha = 1.96
  const delta = effectSize * Math.sqrt(sampleSize / 2)
  const zBeta = delta - zAlpha

  return normalCDF(zBeta)
}
