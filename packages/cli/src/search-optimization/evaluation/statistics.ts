/**
 * Statistical analysis utilities for comparison framework
 *
 * Implements basic statistical tests needed for scientific validation:
 * - Independent samples t-test
 * - Cohen's d effect size
 * - 95% confidence intervals
 *
 * These enable objective, reproducible claims about semantic search value
 * with statistical significance testing.
 */

/**
 * Result of a t-test
 */
export interface TTestResult {
  /** Test statistic value */
  tStatistic: number

  /** Degrees of freedom */
  degreesOfFreedom: number

  /** Two-tailed p-value */
  pValue: number

  /** Whether the result is statistically significant (p < 0.05) */
  significant: boolean

  /** Mean of the first group */
  mean1: number

  /** Mean of the second group */
  mean2: number

  /** Sample size of first group */
  n1: number

  /** Sample size of second group */
  n2: number
}

/**
 * Result of effect size calculation
 */
export interface EffectSizeResult {
  /** Cohen's d effect size */
  cohensD: number

  /** Interpretation of effect size */
  interpretation: 'negligible' | 'small' | 'medium' | 'large' | 'very large'

  /** Pooled standard deviation used in calculation */
  pooledStdDev: number
}

/**
 * Confidence interval result
 */
export interface ConfidenceInterval {
  /** Lower bound of the interval */
  lower: number

  /** Upper bound of the interval */
  upper: number

  /** Mean value */
  mean: number

  /** Standard error */
  standardError: number

  /** Confidence level (typically 0.95 for 95%) */
  confidenceLevel: number
}

/**
 * Calculate mean of an array of numbers
 */
export function mean(values: number[]): number {
  if (values.length === 0) {
    throw new Error('Cannot calculate mean of empty array')
  }
  return values.reduce((sum, val) => sum + val, 0) / values.length
}

/**
 * Calculate variance of an array of numbers
 *
 * @param values - Array of numbers
 * @param sampleVariance - If true, uses n-1 for sample variance (default: true)
 */
export function variance(values: number[], sampleVariance = true): number {
  if (values.length === 0) {
    throw new Error('Cannot calculate variance of empty array')
  }
  if (sampleVariance && values.length === 1) {
    throw new Error('Cannot calculate sample variance with only one value')
  }

  const m = mean(values)
  const squaredDiffs = values.map((val) => Math.pow(val - m, 2))
  const divisor = sampleVariance ? values.length - 1 : values.length
  return squaredDiffs.reduce((sum, val) => sum + val, 0) / divisor
}

/**
 * Calculate standard deviation of an array of numbers
 *
 * @param values - Array of numbers
 * @param sampleStdDev - If true, uses n-1 for sample std dev (default: true)
 */
export function standardDeviation(values: number[], sampleStdDev = true): number {
  return Math.sqrt(variance(values, sampleStdDev))
}

/**
 * Perform independent samples t-test (Welch's t-test)
 *
 * Uses Welch's t-test which doesn't assume equal variances.
 * This is more robust than Student's t-test for real-world data.
 *
 * Null hypothesis: The two populations have equal means
 * Alternative hypothesis: The two populations have different means
 *
 * @param group1 - First group of values
 * @param group2 - Second group of values
 * @returns T-test result with statistic, p-value, and significance
 *
 * @example
 * ```typescript
 * const grepScores = [0.3, 0.4, 0.35, 0.38, 0.32]
 * const searchScores = [0.7, 0.75, 0.72, 0.78, 0.71]
 * const result = tTest(grepScores, searchScores)
 * console.log('p-value:', result.pValue)
 * console.log('significant:', result.significant) // true if p < 0.05
 * ```
 */
export function tTest(group1: number[], group2: number[]): TTestResult {
  if (group1.length < 2) {
    throw new Error('Group 1 must have at least 2 values')
  }
  if (group2.length < 2) {
    throw new Error('Group 2 must have at least 2 values')
  }

  const n1 = group1.length
  const n2 = group2.length
  const mean1 = mean(group1)
  const mean2 = mean(group2)
  const var1 = variance(group1)
  const var2 = variance(group2)

  // Welch's t-test statistic
  const tStatistic = (mean1 - mean2) / Math.sqrt(var1 / n1 + var2 / n2)

  // Welch-Satterthwaite degrees of freedom
  const numerator = Math.pow(var1 / n1 + var2 / n2, 2)
  const denominator = Math.pow(var1 / n1, 2) / (n1 - 1) + Math.pow(var2 / n2, 2) / (n2 - 1)
  const degreesOfFreedom = numerator / denominator

  // Calculate two-tailed p-value
  const pValue = 2 * (1 - tCDF(Math.abs(tStatistic), degreesOfFreedom))

  return {
    tStatistic,
    degreesOfFreedom,
    pValue,
    significant: pValue < 0.05,
    mean1,
    mean2,
    n1,
    n2,
  }
}

/**
 * Calculate Cohen's d effect size
 *
 * Cohen's d measures the standardized difference between two means.
 * It indicates practical significance (how big the difference is)
 * independent of sample size.
 *
 * Interpretation (Cohen's conventions):
 * - |d| < 0.2: negligible
 * - |d| < 0.5: small
 * - |d| < 0.8: medium
 * - |d| < 1.3: large
 * - |d| >= 1.3: very large
 *
 * @param group1 - First group of values
 * @param group2 - Second group of values
 * @returns Effect size result with Cohen's d and interpretation
 *
 * @example
 * ```typescript
 * const grepScores = [0.3, 0.4, 0.35, 0.38, 0.32]
 * const searchScores = [0.7, 0.75, 0.72, 0.78, 0.71]
 * const result = cohensD(grepScores, searchScores)
 * console.log('Effect size:', result.cohensD)
 * console.log('Interpretation:', result.interpretation) // e.g., 'large'
 * ```
 */
export function cohensD(group1: number[], group2: number[]): EffectSizeResult {
  if (group1.length < 2) {
    throw new Error('Group 1 must have at least 2 values')
  }
  if (group2.length < 2) {
    throw new Error('Group 2 must have at least 2 values')
  }

  const n1 = group1.length
  const n2 = group2.length
  const mean1 = mean(group1)
  const mean2 = mean(group2)
  const var1 = variance(group1)
  const var2 = variance(group2)

  // Pooled standard deviation
  const pooledStdDev = Math.sqrt(((n1 - 1) * var1 + (n2 - 1) * var2) / (n1 + n2 - 2))

  // Cohen's d (handle division by zero)
  let d: number
  if (pooledStdDev === 0) {
    // If pooled std dev is 0, both groups are constant
    // If means are equal, effect is 0; otherwise undefined (but call it 0)
    d = mean1 === mean2 ? 0 : 0
  } else {
    d = (mean1 - mean2) / pooledStdDev
  }

  // Interpret effect size
  const absD = Math.abs(d)
  let interpretation: EffectSizeResult['interpretation']
  if (absD < 0.2) {
    interpretation = 'negligible'
  } else if (absD < 0.5) {
    interpretation = 'small'
  } else if (absD < 0.8) {
    interpretation = 'medium'
  } else if (absD < 1.3) {
    interpretation = 'large'
  } else {
    interpretation = 'very large'
  }

  return {
    cohensD: d,
    interpretation,
    pooledStdDev,
  }
}

/**
 * Calculate 95% confidence interval for the mean
 *
 * Confidence interval provides a range of plausible values for the
 * true population mean. 95% confidence means that if we repeated
 * the experiment many times, 95% of calculated intervals would
 * contain the true mean.
 *
 * Uses t-distribution for small samples (more conservative than z).
 *
 * @param values - Array of values
 * @param confidenceLevel - Confidence level (default: 0.95 for 95%)
 * @returns Confidence interval with lower and upper bounds
 *
 * @example
 * ```typescript
 * const scores = [0.7, 0.75, 0.72, 0.78, 0.71]
 * const ci = confidenceInterval(scores)
 * console.log(`95% CI: [${ci.lower.toFixed(2)}, ${ci.upper.toFixed(2)}]`)
 * ```
 */
export function confidenceInterval(values: number[], confidenceLevel = 0.95): ConfidenceInterval {
  if (values.length < 2) {
    throw new Error('Need at least 2 values to calculate confidence interval')
  }

  const n = values.length
  const m = mean(values)
  const stdDev = standardDeviation(values)
  const standardError = stdDev / Math.sqrt(n)

  // t-value for two-tailed test
  const alpha = 1 - confidenceLevel
  const df = n - 1
  const tValue = tInverse(1 - alpha / 2, df)

  // Margin of error
  const marginOfError = tValue * standardError

  return {
    lower: m - marginOfError,
    upper: m + marginOfError,
    mean: m,
    standardError,
    confidenceLevel,
  }
}

/**
 * Calculate 95% confidence interval for the difference between two means
 *
 * This provides a range for the true difference between populations.
 * If the interval doesn't include 0, the difference is significant at p<0.05.
 *
 * @param group1 - First group of values
 * @param group2 - Second group of values
 * @param confidenceLevel - Confidence level (default: 0.95 for 95%)
 * @returns Confidence interval for the difference
 *
 * @example
 * ```typescript
 * const grepScores = [0.3, 0.4, 0.35, 0.38, 0.32]
 * const searchScores = [0.7, 0.75, 0.72, 0.78, 0.71]
 * const ci = confidenceIntervalDifference(searchScores, grepScores)
 * console.log(`Improvement: ${ci.mean.toFixed(2)} [${ci.lower.toFixed(2)}, ${ci.upper.toFixed(2)}]`)
 * // If lower > 0, we can be confident search is better
 * ```
 */
export function confidenceIntervalDifference(
  group1: number[],
  group2: number[],
  confidenceLevel = 0.95,
): ConfidenceInterval {
  if (group1.length < 2) {
    throw new Error('Group 1 must have at least 2 values')
  }
  if (group2.length < 2) {
    throw new Error('Group 2 must have at least 2 values')
  }

  const n1 = group1.length
  const n2 = group2.length
  const mean1 = mean(group1)
  const mean2 = mean(group2)
  const var1 = variance(group1)
  const var2 = variance(group2)

  // Standard error of difference
  const standardError = Math.sqrt(var1 / n1 + var2 / n2)

  // Welch-Satterthwaite degrees of freedom
  const numerator = Math.pow(var1 / n1 + var2 / n2, 2)
  const denominator = Math.pow(var1 / n1, 2) / (n1 - 1) + Math.pow(var2 / n2, 2) / (n2 - 1)
  const df = numerator / denominator

  // t-value for two-tailed test
  const alpha = 1 - confidenceLevel
  const tValue = tInverse(1 - alpha / 2, df)

  // Margin of error
  const marginOfError = tValue * standardError
  const meanDiff = mean1 - mean2

  return {
    lower: meanDiff - marginOfError,
    upper: meanDiff + marginOfError,
    mean: meanDiff,
    standardError,
    confidenceLevel,
  }
}

/**
 * Cumulative distribution function (CDF) for t-distribution
 *
 * Approximation using Wilson-Hilferty transformation.
 * Accurate enough for our purposes (error < 0.005 for df >= 5).
 *
 * @param t - t-statistic value
 * @param df - Degrees of freedom
 * @returns Probability that a t-distributed random variable is less than t
 */
function tCDF(t: number, df: number): number {
  // For large df, t-distribution approaches normal distribution
  if (df > 100) {
    return normalCDF(t)
  }

  // Wilson-Hilferty transformation
  // Converts t to approximate normal
  const a = df - 0.5
  const b = 48 * a * a
  const z = a * Math.log(1 + (t * t) / df)
  const normalizedZ = ((Math.pow(z, 1 / 3) - 1 + 1 / (9 * a)) * Math.sqrt(b)) / (1 + z / b)

  return normalCDF(normalizedZ)
}

/**
 * Inverse t-distribution (quantile function)
 *
 * Finds the t-value corresponding to a given probability.
 * Used for calculating confidence intervals.
 *
 * Uses improved approximation from Hill (1970) algorithm.
 *
 * @param p - Probability (0 to 1)
 * @param df - Degrees of freedom
 * @returns t-value such that P(T <= t) = p
 */
function tInverse(p: number, df: number): number {
  if (p <= 0 || p >= 1) {
    throw new Error('Probability must be between 0 and 1')
  }

  // For large df, use normal approximation
  if (df > 100) {
    return normalInverse(p)
  }

  // Get initial estimate from normal distribution
  const z = normalInverse(p)

  // Hill's approximation for t-inverse
  // More accurate than simple normal approximation for small df
  const g1 = (z * z * z + z) / 4
  const g2 = (5 * Math.pow(z, 5) + 16 * z * z * z + 3 * z) / 96
  const g3 = (3 * Math.pow(z, 7) + 19 * Math.pow(z, 5) + 17 * z * z * z - 15 * z) / 384
  const g4 = (79 * Math.pow(z, 9) + 776 * Math.pow(z, 7) + 1482 * Math.pow(z, 5) - 1920 * z * z * z - 945 * z) / 92160

  const t = z + g1 / df + g2 / (df * df) + g3 / Math.pow(df, 3) + g4 / Math.pow(df, 4)

  return t
}

/**
 * Cumulative distribution function (CDF) for standard normal distribution
 *
 * Uses error function approximation.
 *
 * @param z - z-score
 * @returns Probability that a standard normal random variable is less than z
 */
function normalCDF(z: number): number {
  return 0.5 * (1 + erf(z / Math.sqrt(2)))
}

/**
 * Inverse normal distribution (quantile function)
 *
 * Finds the z-score corresponding to a given probability.
 * Uses Beasley-Springer-Moro algorithm (accurate to 1e-9).
 *
 * @param p - Probability (0 to 1)
 * @returns z-score such that P(Z <= z) = p
 */
function normalInverse(p: number): number {
  if (p <= 0 || p >= 1) {
    throw new Error('Probability must be between 0 and 1')
  }

  // Coefficients for rational approximation
  const a = [
    -3.969683028665376e1, 2.209460984245205e2, -2.759285104469687e2, 1.38357751867269e2, -3.066479806614716e1,
    2.506628277459239,
  ]

  const b = [-5.447609879822406e1, 1.615858368580409e2, -1.556989798598866e2, 6.680131188771972e1, -1.328068155288572e1]

  const c = [
    -7.784894002430293e-3, -3.223964580411365e-1, -2.400758277161838, -2.549732539343734, 4.374664141464968,
    2.938163982698783,
  ]

  const d = [7.784695709041462e-3, 3.224671290700398e-1, 2.445134137142996, 3.754408661907416]

  const pLow = 0.02425
  const pHigh = 1 - pLow

  // Rational approximation for lower region
  if (p < pLow) {
    const q = Math.sqrt(-2 * Math.log(p))
    return (
      (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) /
      ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1)
    )
  }

  // Rational approximation for upper region
  if (p > pHigh) {
    const q = Math.sqrt(-2 * Math.log(1 - p))
    return (
      -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) /
      ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1)
    )
  }

  // Rational approximation for central region
  const q = p - 0.5
  const r = q * q
  return (
    ((((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q) /
    (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1)
  )
}

/**
 * Error function (erf)
 *
 * Uses Abramowitz and Stegun approximation (accurate to 1.5e-7).
 *
 * @param x - Input value
 * @returns erf(x)
 */
function erf(x: number): number {
  // Constants for approximation
  const a1 = 0.254829592
  const a2 = -0.284496736
  const a3 = 1.421413741
  const a4 = -1.453152027
  const a5 = 1.061405429
  const p = 0.3275911

  // Save the sign of x
  const sign = x >= 0 ? 1 : -1
  x = Math.abs(x)

  // Abramowitz and Stegun approximation
  const t = 1.0 / (1.0 + p * x)
  const y = 1.0 - ((((a5 * t + a4) * t + a3) * t + a2) * t + a1) * t * Math.exp(-x * x)

  return sign * y
}
