<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

// Scenario: @var array type with matching element types
// Expected: No errors - all elements match the declared type

class TestArrayElementsMatch {
    /**
     * @var int[]
     */
    private $integers = [1, 2, 3];

    /**
     * @var string[]
     */
    private $strings = ["hello", "world"];

    /**
     * @var bool[]
     */
    private $flags = [true, false, true];
}
