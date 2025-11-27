<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

// Scenario: @var object array type with correctly typed object elements
// Expected: No errors

class User {
    public $name;
}

class Admin {
    public $role;
}

class TestObjectArrayMatches {
    /**
     * @var User[]
     */
    private $users = [new User(), new User()];

    /**
     * @var Admin[]
     */
    private $admins = [new Admin()];
}
