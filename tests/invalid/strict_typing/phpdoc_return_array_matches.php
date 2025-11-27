<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_value_check

// Scenario: @return array type with correctly typed array elements
// Expected: No errors

class TestReturnArrayMatches {
    /**
     * @return int[]
     */
    function getIntegers(): array {
        return [1, 2, 3];
    }

    /**
     * @return string[]
     */
    function getStrings(): array {
        return ["hello", "world"];
    }

    /**
     * @return bool[]
     */
    function getFlags(): array {
        return [true, false, true];
    }
}
