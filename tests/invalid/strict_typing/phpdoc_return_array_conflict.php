<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_value_check

// Scenario: @return array type with mismatched element types
// Expected: Errors on lines 11, 19, 27

class TestReturnArrayConflict {
    /**
     * @return int[]
     */
    function getIntegers(): array {
        return [1, "string", 3]; // Error: string in int array
    }

    /**
     * @return string[]
     */
    function getStrings(): array {
        return ["hello", 123]; // Error: int in string array
    }

    /**
     * @return bool[]
     */
    function getFlags(): array {
        return [true, "false"]; // Error: string in bool array
    }
}
