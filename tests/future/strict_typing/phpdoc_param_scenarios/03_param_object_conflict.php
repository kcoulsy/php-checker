<?php
// php-checker-test: only-rules=strict_typing/phpdoc_param_check

// Scenario: @param object type conflicts with native object type
// Expected: Error on line 11

class User {}
class Admin {}

/**
 * @param User $user
 */
function processUser(Admin $user) {
    // Error: @param type 'User' conflicts with native type hint 'Admin'
}
