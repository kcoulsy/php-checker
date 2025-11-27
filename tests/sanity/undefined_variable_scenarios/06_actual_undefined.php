<?php
// php-checker-test: only-rules=sanity/undefined_variable

// Scenario: Actual undefined variables should still be caught
// Expected: Error on line 8 for $undefinedVar

function test() {
    echo $undefinedVar;
}
