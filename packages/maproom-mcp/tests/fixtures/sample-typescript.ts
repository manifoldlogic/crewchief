/**
 * Sample TypeScript file for E2E testing
 * This file contains typical TypeScript patterns
 */

export interface User {
  id: number
  name: string
  email: string
  createdAt: Date
}

export class UserService {
  private users: Map<number, User>

  constructor() {
    this.users = new Map()
  }

  /**
   * Create a new user
   * @param name - User's full name
   * @param email - User's email address
   * @returns The created user
   */
  async createUser(name: string, email: string): Promise<User> {
    const id = this.users.size + 1
    const user: User = {
      id,
      name,
      email,
      createdAt: new Date(),
    }
    this.users.set(id, user)
    return user
  }

  /**
   * Find user by ID
   * @param id - User ID to search for
   * @returns User if found, undefined otherwise
   */
  async findById(id: number): Promise<User | undefined> {
    return this.users.get(id)
  }

  /**
   * Delete user by ID
   * @param id - User ID to delete
   * @returns True if deleted, false if not found
   */
  async deleteUser(id: number): Promise<boolean> {
    return this.users.delete(id)
  }

  /**
   * List all users
   * @returns Array of all users
   */
  async listUsers(): Promise<User[]> {
    return Array.from(this.users.values())
  }
}

export function validateEmail(email: string): boolean {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
  return emailRegex.test(email)
}

export function formatUserName(user: User): string {
  return `${user.name} <${user.email}>`
}
