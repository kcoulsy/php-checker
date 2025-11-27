<?php
// php-checker-test: only-rules=sanity/undefined_variable

// Scenario: Class properties should not be flagged as undefined
// Expected: No errors

class User {
    public $name = "John";
    public $email = null;
    private $password;
    protected $roles = [];
}
