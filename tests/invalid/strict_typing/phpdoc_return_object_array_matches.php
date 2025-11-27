<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_value_check

// Scenario: @return object array type with correctly typed object elements
// Expected: No errors

class User {
    public $name;
}

class Admin {
    public $role;
}

class TestReturnObjectArrayMatches {
    /**
     * @return User[]
     */
    function getUsers(): array {
        return [new User(), new User()];
    }

    /**
     * @return Admin[]
     */
    function getAdmins(): array {
        return [new Admin()];
    }
}
