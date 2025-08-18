/**
 * Example Unit Test
 * 
 * Demonstrates best practices for unit testing in the CrewChief Web UI project.
 * This file serves as a template and reference for writing effective unit tests.
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// Example of testing a utility function
function calculateTotal(items: { price: number; quantity: number }[]): number {
  if (!items || items.length === 0) {
    return 0;
  }
  
  return items.reduce((total, item) => {
    if (typeof item.price !== 'number' || typeof item.quantity !== 'number') {
      throw new Error('Invalid item: price and quantity must be numbers');
    }
    if (!isFinite(item.price) || !isFinite(item.quantity)) {
      throw new Error('Invalid item: price and quantity must be finite numbers');
    }
    return total + (item.price * item.quantity);
  }, 0);
}

// Example of testing a class
class ShoppingCart {
  private items: { id: string; price: number; quantity: number }[] = [];

  addItem(id: string, price: number, quantity: number = 1): void {
    if (price < 0) {
      throw new Error('Price cannot be negative');
    }
    
    if (quantity <= 0) {
      throw new Error('Quantity must be positive');
    }

    const existingItem = this.items.find(item => item.id === id);
    if (existingItem) {
      existingItem.quantity += quantity;
    } else {
      this.items.push({ id, price, quantity });
    }
  }

  removeItem(id: string): boolean {
    const index = this.items.findIndex(item => item.id === id);
    if (index !== -1) {
      this.items.splice(index, 1);
      return true;
    }
    return false;
  }

  getTotal(): number {
    return calculateTotal(this.items);
  }

  getItemCount(): number {
    return this.items.reduce((count, item) => count + item.quantity, 0);
  }

  clear(): void {
    this.items = [];
  }

  getItems(): { id: string; price: number; quantity: number }[] {
    return [...this.items]; // Return a copy to prevent external modification
  }
}

describe('Example Unit Tests', () => {
  describe('calculateTotal utility function', () => {
    it('calculates total for empty array', () => {
      expect(calculateTotal([])).toBe(0);
    });

    it('calculates total for null/undefined input', () => {
      expect(calculateTotal(null as any)).toBe(0);
      expect(calculateTotal(undefined as any)).toBe(0);
    });

    it('calculates total for single item', () => {
      const items = [{ price: 10, quantity: 2 }];
      expect(calculateTotal(items)).toBe(20);
    });

    it('calculates total for multiple items', () => {
      const items = [
        { price: 10, quantity: 2 },
        { price: 5, quantity: 3 },
        { price: 20, quantity: 1 }
      ];
      expect(calculateTotal(items)).toBe(55); // (10*2) + (5*3) + (20*1)
    });

    it('handles decimal prices correctly', () => {
      const items = [
        { price: 9.99, quantity: 2 },
        { price: 15.50, quantity: 1 }
      ];
      expect(calculateTotal(items)).toBeCloseTo(35.48);
    });

    it('throws error for invalid input', () => {
      const invalidItems = [
        { price: 'invalid', quantity: 2 }
      ] as any;
      
      expect(() => calculateTotal(invalidItems)).toThrow('Invalid item: price and quantity must be numbers');
    });
  });

  describe('ShoppingCart class', () => {
    let cart: ShoppingCart;

    beforeEach(() => {
      cart = new ShoppingCart();
    });

    describe('addItem method', () => {
      it('adds new item to empty cart', () => {
        cart.addItem('item1', 10, 2);
        
        expect(cart.getItems()).toHaveLength(1);
        expect(cart.getItems()[0]).toEqual({
          id: 'item1',
          price: 10,
          quantity: 2
        });
      });

      it('increases quantity when adding existing item', () => {
        cart.addItem('item1', 10, 2);
        cart.addItem('item1', 10, 3);
        
        expect(cart.getItems()).toHaveLength(1);
        expect(cart.getItems()[0].quantity).toBe(5);
      });

      it('uses default quantity of 1 when not specified', () => {
        cart.addItem('item1', 10);
        
        expect(cart.getItems()[0].quantity).toBe(1);
      });

      it('throws error for negative price', () => {
        expect(() => cart.addItem('item1', -5, 1)).toThrow('Price cannot be negative');
      });

      it('throws error for zero or negative quantity', () => {
        expect(() => cart.addItem('item1', 10, 0)).toThrow('Quantity must be positive');
        expect(() => cart.addItem('item1', 10, -1)).toThrow('Quantity must be positive');
      });
    });

    describe('removeItem method', () => {
      beforeEach(() => {
        cart.addItem('item1', 10, 2);
        cart.addItem('item2', 5, 1);
      });

      it('removes existing item and returns true', () => {
        const result = cart.removeItem('item1');
        
        expect(result).toBe(true);
        expect(cart.getItems()).toHaveLength(1);
        expect(cart.getItems()[0].id).toBe('item2');
      });

      it('returns false when item does not exist', () => {
        const result = cart.removeItem('nonexistent');
        
        expect(result).toBe(false);
        expect(cart.getItems()).toHaveLength(2);
      });
    });

    describe('getTotal method', () => {
      it('returns 0 for empty cart', () => {
        expect(cart.getTotal()).toBe(0);
      });

      it('calculates total for single item', () => {
        cart.addItem('item1', 10, 2);
        expect(cart.getTotal()).toBe(20);
      });

      it('calculates total for multiple items', () => {
        cart.addItem('item1', 10, 2);
        cart.addItem('item2', 5, 3);
        expect(cart.getTotal()).toBe(35);
      });
    });

    describe('getItemCount method', () => {
      it('returns 0 for empty cart', () => {
        expect(cart.getItemCount()).toBe(0);
      });

      it('returns total quantity of all items', () => {
        cart.addItem('item1', 10, 2);
        cart.addItem('item2', 5, 3);
        expect(cart.getItemCount()).toBe(5);
      });
    });

    describe('clear method', () => {
      it('removes all items from cart', () => {
        cart.addItem('item1', 10, 2);
        cart.addItem('item2', 5, 3);
        
        cart.clear();
        
        expect(cart.getItems()).toHaveLength(0);
        expect(cart.getTotal()).toBe(0);
        expect(cart.getItemCount()).toBe(0);
      });
    });

    describe('getItems method', () => {
      it('returns copy of items array', () => {
        cart.addItem('item1', 10, 2);
        const items = cart.getItems();
        
        // Modify the returned array
        items.push({ id: 'hacker', price: 999, quantity: 1 });
        
        // Original cart should be unaffected
        expect(cart.getItems()).toHaveLength(1);
        expect(cart.getItems()[0].id).toBe('item1');
      });
    });
  });

  describe('Mocking and Spies', () => {
    it('demonstrates function mocking', () => {
      const mockCallback = vi.fn();
      const mockCalculator = {
        add: vi.fn(),
        multiply: vi.fn()
      };

      // Set up mock return values
      mockCalculator.add.mockReturnValue(10);
      mockCalculator.multiply.mockReturnValue(50);

      // Use the mocks
      const sum = mockCalculator.add(5, 5);
      const product = mockCalculator.multiply(5, 10);

      // Verify the results
      expect(sum).toBe(10);
      expect(product).toBe(50);

      // Verify the mocks were called correctly
      expect(mockCalculator.add).toHaveBeenCalledWith(5, 5);
      expect(mockCalculator.multiply).toHaveBeenCalledWith(5, 10);
      expect(mockCalculator.add).toHaveBeenCalledTimes(1);
      expect(mockCalculator.multiply).toHaveBeenCalledTimes(1);
    });

    it('demonstrates spy functionality', () => {
      const originalConsole = console.log;
      const consoleSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      // Function that logs something
      function logMessage(message: string) {
        console.log(`Message: ${message}`);
      }

      logMessage('Hello World');

      // Verify the spy was called
      expect(consoleSpy).toHaveBeenCalledWith('Message: Hello World');
      expect(consoleSpy).toHaveBeenCalledTimes(1);

      // Restore the original function
      consoleSpy.mockRestore();
    });
  });

  describe('Async Operations', () => {
    // Mock async function
    async function fetchUserData(id: string): Promise<{ id: string; name: string }> {
      if (id === 'error') {
        throw new Error('User not found');
      }
      
      // Simulate API delay
      await new Promise(resolve => setTimeout(resolve, 10));
      
      return { id, name: `User ${id}` };
    }

    it('handles successful async operations', async () => {
      const userData = await fetchUserData('123');
      
      expect(userData).toEqual({
        id: '123',
        name: 'User 123'
      });
    });

    it('handles async errors', async () => {
      await expect(fetchUserData('error')).rejects.toThrow('User not found');
    });

    it('demonstrates async mocking', async () => {
      const mockFetch = vi.fn();
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ id: '456', name: 'Mock User' })
      });

      // Simulate using the mock
      const response = await mockFetch('/api/users/456');
      const data = await response.json();

      expect(data).toEqual({ id: '456', name: 'Mock User' });
      expect(mockFetch).toHaveBeenCalledWith('/api/users/456');
    });
  });

  describe('Edge Cases and Error Handling', () => {
    it('handles boundary values', () => {
      expect(calculateTotal([{ price: 0, quantity: 1 }])).toBe(0);
      expect(calculateTotal([{ price: Number.MAX_SAFE_INTEGER, quantity: 1 }])).toBe(Number.MAX_SAFE_INTEGER);
    });

    it('handles special number values', () => {
      // These should throw errors due to validation
      expect(() => calculateTotal([{ price: NaN, quantity: 1 }] as any)).toThrow();
      expect(() => calculateTotal([{ price: Infinity, quantity: 1 }] as any)).toThrow();
    });
  });
});