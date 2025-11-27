<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_value_check

// Scenario: @return object array type with mismatched object types
// Expected: Errors on lines 19, 27

class User {
    public $name;
}

class Admin {
    public $role;
}

class TestReturnObjectArrayConflict {
    /**
     * @return User[]
     */
    function getUsers(): array {
        return [new User(), new Admin()]; // Error: Admin in User array
    }

    /**
     * @return Admin[]
     */
    function getAdmins(): array {
        return [new User(), new Admin()]; // Error: User in Admin array
    }
}
