import chalk from 'chalk';

export const logger = {
  info: (...args: unknown[]): void => console.log(chalk.blue('[info]'), ...args),
  success: (...args: unknown[]): void => console.log(chalk.green('[ok]'), ...args),
  warn: (...args: unknown[]): void => console.warn(chalk.yellow('[warn]'), ...args),
  error: (...args: unknown[]): void => console.error(chalk.red('[err]'), ...args)
};

