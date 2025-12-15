export function add(a: number, b: number): number {
    return a + b;
}

export function multiply(a: number, b: number): number {
    return a * b;
}

export function calculate(x: number, y: number): number {
    const sum = add(x, y);
    const product = multiply(x, y);
    return sum + product;
}
