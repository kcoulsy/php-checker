<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_value_check

// Scenario: @return type conflicts with actual return values
// Expected: Errors on lines 11, 19, 27

class TestReturnValueConflict {
    /**
     * @return int
     */
    function getNumber(): int {
        return "not a number"; // Error: string instead of int
    }

    /**
     * @return string
     */
    function getName(): string {
        return 123; // Error: int instead of string
    }

    /**
     * @return bool
     */
    function isValid(): bool {
        return "yes"; // Error: string instead of bool
    }
}
