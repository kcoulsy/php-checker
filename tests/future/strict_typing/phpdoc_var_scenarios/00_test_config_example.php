<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

// Scenario: Test configuration example - only test phpdoc_var_check rule
// Expected: Error on line 9 (type mismatch), but no errors from other rules

class TestConfigExample {
    /** @var string */
    private $name = 123;  // Error: int assigned to string property

    private $undefinedVar = $missingVariable;  // Would error with undefined_variable rule, but that's disabled
}
