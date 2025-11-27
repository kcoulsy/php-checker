<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

// Scenario: @var array type with mismatched element types
// Expected: Errors on lines 11, 17, 23

class TestArrayElementsConflict {
    /**
     * @var int[]
     */
    private $integers = [1, "string", 3]; // Error: string in int array

    /**
     * @var string[]
     */
    private $strings = ["hello", 123]; // Error: int in string array

    /**
     * @var bool[]
     */
    private $flags = [true, "false"]; // Error: string in bool array
}
