<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

// Scenario: @var with simple array types (int[], string[])
// Expected: No errors for empty arrays (we'll validate elements later)

class TestSimpleArrays {
    /**
     * @var int[]
     */
    private $integers = [];

    /**
     * @var string[]
     */
    private $strings = [];

    /**
     * @var bool[]
     */
    private $flags = [];
}
