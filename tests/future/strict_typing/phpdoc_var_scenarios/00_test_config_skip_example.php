<?php
// php-checker-test: skip-rules=sanity/undefined_variable

// Scenario: Test configuration example - skip undefined_variable rule
// Expected: Error on line 9 (type mismatch), but no undefined variable error

class TestConfigSkipExample {
    /** @var string */
    private $name = 123;  // Error: int assigned to string property

    private $test = $missingVariable;  // Would error with undefined_variable rule, but that's skipped
}
