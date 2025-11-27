<?php
// php-checker-test: only-rules=strict_typing/phpdoc_param_check

// Scenario: @param union type conflicts with native union type hint
// Expected: Errors on lines 9, 16, 23

class TestParamUnionConflict {
    /**
     * @param int|string $value
     */
    function wrongUnion(int|bool $value) {
        // Error - bool is not string
    }

    /**
     * @param int|string $value
     */
    function differentUnion(string|float $value) {
        // Error - types don't match
    }

    /**
     * @param User|Admin $obj
     */
    function wrongObjectUnion(User|Guest $obj) {
        // Error - Guest is not Admin
    }
}
