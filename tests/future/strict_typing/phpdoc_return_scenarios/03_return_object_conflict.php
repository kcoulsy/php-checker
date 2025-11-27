<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_check

// Scenario: @return object type conflicts with native return type
// Expected: Error on line 11

class User {}
class Admin {}

/**
 * @return User
 */
function getAdmin(): Admin {
    return new Admin();
}
