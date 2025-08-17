import chalk from 'chalk'

interface SubshellMessageOptions {
  targetBranch: string
  targetDirectory: string
  sourceBranch: string
  sourceDirectory: string
  shell: string
}

export function displaySubshellMessage(options: SubshellMessageOptions): void {
  const { targetBranch, targetDirectory, sourceBranch, sourceDirectory, shell } = options
  
  const boxWidth = 70
  const borderColor = chalk.cyan
  
  // Helper to create a padded line
  const makeLine = (content: string): string => {
    // Remove ANSI codes to get actual string length
    const visibleLength = content.replace(/\x1b\[[0-9;]*m/g, '').length
    const padding = boxWidth - 2 - visibleLength
    return borderColor('║') + ' ' + content + ' '.repeat(Math.max(0, padding)) + ' ' + borderColor('║')
  }
  
  console.log()
  console.log(borderColor('╔' + '═'.repeat(boxWidth) + '╗'))
  console.log(makeLine(chalk.bold.white('ENTERING WORKTREE SUBSHELL')))
  console.log(makeLine(''))
  console.log(makeLine(chalk.white('Switching to branch: ') + chalk.green.bold(targetBranch)))
  console.log(makeLine(chalk.white('Directory: ') + chalk.white(targetDirectory)))
  console.log(makeLine(''))
  console.log(makeLine(chalk.yellow('Type ') + chalk.bold.yellow('"exit"') + chalk.yellow(' to return to:')))
  console.log(makeLine(chalk.gray(`  • Original directory: ${sourceDirectory}`)))
  console.log(makeLine(chalk.gray(`  • Original branch: ${sourceBranch}`)))
  console.log(makeLine(chalk.gray(`  • Parent shell session`)))
  console.log(borderColor('╚' + '═'.repeat(boxWidth) + '╝'))
  console.log()
}