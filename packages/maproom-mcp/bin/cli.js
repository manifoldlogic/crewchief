#!/usr/bin/env node
// Small CLI shim that executes the built JS with Node, ensuring it runs under npx without ESM shebang issues.
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)
const entry = path.join(__dirname, '..', 'dist', 'index.js')
// Dynamically import to support ESM without requiring transpilation here
await import(pathToFileURL(entry).href)


