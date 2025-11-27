<?php
// php-checker-test: only-rules=strict_typing/phpdoc_param_check

// Scenario: @param object type matches native object type
// Expected: No errors

class User {}

/**
 * @param User $user
 */
function processUser(User $user) {
    // No error: types match
}
