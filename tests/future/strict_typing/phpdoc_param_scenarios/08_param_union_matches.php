<?php
// php-checker-test: only-rules=strict_typing/phpdoc_param_check

// Scenario: @param union type matches native union type hint
// Expected: No errors

class TestParamUnionMatches {
    /**
     * @param int|string $value
     */
    function acceptsIntOrString(int|string $value) {
        // OK - PHPDoc matches native type
    }

    /**
     * @param int|string|bool $value
     */
    function acceptsMultipleTypes(int|string|bool $value) {
        // OK - PHPDoc matches native union type
    }

    /**
     * @param User|Admin $obj
     */
    function acceptsUserOrAdmin(User|Admin $obj) {
        // OK - union of objects
    }
}
