<?php

// Function without return type - should trigger warning
function noReturnType() {
    return 42;
}

// Function with void return type - should be OK
function withVoidReturnType(): void {
    // no return needed
}

// Function with int return type - should be OK
function withIntReturnType(): int {
    return 42;
}

// Function with string return type - should be OK
function withStringReturnType(): string {
    return "hello";
}

noReturnType();
withVoidReturnType();
withIntReturnType();
withStringReturnType();
