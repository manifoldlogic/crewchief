// Nested calls
function outer() {
    inner();
}

function inner() {
    helper();
}

function helper() {
    return 42;
}

// Higher-order functions
function map(fn: Function, arr: number[]) {
    return arr.map(fn);
}

function double(x: number) {
    return x * 2;
}

// Arrow functions (inline - may not create edges)
const process = (x: number) => {
    return double(x);
};

// Multiple calls
function orchestrate() {
    outer();
    inner();
    helper();
    const result = map(double, [1, 2, 3]);
    return result;
}

orchestrate();
