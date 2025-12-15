// Converted to standalone functions to match Phase 1 capabilities
// (Class method extraction not yet implemented)

function add(a: number, b: number): number {
    return a + b;
}

function subtract(a: number, b: number): number {
    return a - b;
}

function multiply(a: number, b: number): number {
    const sum = add(a, a);  // Uses add internally
    return sum * b;
}

function compute(): number {
    const x = add(5, 3);
    const y = multiply(2, 4);
    return subtract(x, y);
}

compute();
