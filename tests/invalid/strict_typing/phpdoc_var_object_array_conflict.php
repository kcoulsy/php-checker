<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

// Scenario: @var object array type with mismatched object types
// Expected: Errors on lines 19, 25

class User {
    public $name;
}

class Admin {
    public $role;
}

class TestObjectArrayConflict {
    /**
     * @var User[]
     */
    private $users = [new User(), new Admin()]; // Error: Admin in User array

    /**
     * @var Admin[]
     */
    private $admins = [new User(), new Admin()]; // Error: User in Admin array
}
