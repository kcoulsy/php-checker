<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_check

// Scenario: @return non-nullable type conflicts with nullable native return type hint
// Expected: Error on line 10

/**
 * @return string
 */
function getName(): ?string {
    return null;
}
