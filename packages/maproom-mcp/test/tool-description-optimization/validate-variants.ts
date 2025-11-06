/**
 * Validation Script - Verify all variants meet requirements
 */

import { readFileSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import type { Variant } from './types.js'
import { validateVariant } from './validator.js'

const variantsDir = join(process.cwd(), 'packages/maproom-mcp/test/tool-description-optimization/variants')

console.log('🔍 Validating all variants...\n')

const variantFiles = readdirSync(variantsDir).filter(f => f.endsWith('.json'))

let allValid = true
const results: Array<{ id: string; valid: boolean; tokenCount: number }> = []

for (const file of variantFiles) {
  const filePath = join(variantsDir, file)
  const variant: Variant = JSON.parse(readFileSync(filePath, 'utf-8'))

  const validation = validateVariant(variant)

  results.push({
    id: variant.id,
    valid: validation.valid,
    tokenCount: validation.tokenCount
  })

  console.log(`📄 ${variant.id}`)
  console.log(`   Name: ${variant.name}`)
  console.log(`   Tokens: ${validation.tokenCount} / 600 ${validation.withinBudget ? '✅' : '❌'}`)
  console.log(`   Valid: ${validation.valid ? '✅' : '❌'}`)

  if (!validation.valid) {
    console.log(`   Errors:`)
    validation.errors.forEach(err => console.log(`     - ${err}`))
    allValid = false
  }

  if (validation.warnings.length > 0) {
    console.log(`   Warnings:`)
    validation.warnings.forEach(warn => console.log(`     - ${warn}`))
  }

  console.log()
}

console.log('📊 Summary:')
console.log(`   Total variants: ${results.length}`)
console.log(`   Valid: ${results.filter(r => r.valid).length}`)
console.log(`   Invalid: ${results.filter(r => !r.valid).length}`)
console.log(`   Token range: ${Math.min(...results.map(r => r.tokenCount))} - ${Math.max(...results.map(r => r.tokenCount))}`)
console.log()

if (allValid) {
  console.log('✅ All variants passed validation!')
  process.exit(0)
} else {
  console.log('❌ Some variants failed validation')
  process.exit(1)
}
