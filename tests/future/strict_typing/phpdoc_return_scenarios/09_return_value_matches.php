<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_value_check

// Scenario: @return type matches actual return values
// Expected: No errors

class TestReturnValueMatches {
    /**
     * @return int
     */
    function getNumber(): int {
        return 42;
    }

    /**
     * @return string
     */
    function getName(): string {
        return "Alice";
    }

    /**
     * @return bool
     */
    function isValid(): bool {
        return true;
    }
}
