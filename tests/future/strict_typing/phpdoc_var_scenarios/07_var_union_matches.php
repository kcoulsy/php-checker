<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

// Scenario: @var union type with values that match one of the union members
// Expected: No errors (currently will fail because we don't support union type compatibility yet)

class TestVarUnionMatches {
    /**
     * @var int|string
     */
    private $intOrString = 123;

    /**
     * @var int|string
     */
    private $intOrString2 = "hello";

    /**
     * @var User|Admin
     */
    private $userOrAdmin;

    /**
     * @var int|string|bool
     */
    private $multiType = true;
}
