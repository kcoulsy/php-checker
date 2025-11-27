<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

// Scenario: @var union type conflicts with assigned value type
// Expected: Errors on lines 9, 15, 21

class TestVarUnionConflict {
    /**
     * @var int|string
     */
    private $wrongType = true;

    /**
     * @var int|string
     */
    private $wrongType2 = 1.5;

    /**
     * @var User|Admin
     */
    private $wrongObject = 123;
}
