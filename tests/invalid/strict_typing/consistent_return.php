<?php

// php-checker-ignore: strict_typing/force_return_type,strict_typing/missing_return,control_flow/unreachable

// Function with inconsistent return types - should trigger error
function inconsistentReturns(bool $flag) {
    if ($flag) {
        return 42;  // returns int
    } else {
        return "hello";  // returns string - inconsistent!
    }
}

// Function with consistent return types - should be OK
function consistentReturns(bool $flag) {
    if ($flag) {
        return 42;
    } else {
        return 24;
    }
}

// Function with mixed types including void - should trigger error
function mixedVoidReturns(bool $flag) {
    if ($flag) {
        return 42;  // returns int
    }
    // implicit void return - inconsistent!
}

// Function with only void returns - should be OK
function voidReturns(bool $flag) {
    if ($flag) {
        return;  // explicit void
    }
    // implicit void return
}

// Function with only one return - should be OK
function singleReturn() {
    return "single";
}

// Function with boolean returns - should be OK
function booleanReturns(bool $flag) {
    if ($flag) {
        return true;
    } else {
        return false;
    }
}

inconsistentReturns(true);
consistentReturns(true);
mixedVoidReturns(true);
voidReturns(true);
singleReturn();
booleanReturns(true);
