<?php

class Test {
    /**
     * @var array{name: string, age: int}
     */
    private $person = ["name" => "Alice", "age" => 30];

    /**
     * @var array{name: string, age: int}
     */
    private $wrong_order = ["age" => 30, "name" => "Bob"]; // Order should not matter

    /**
     * @var array{name: string, age: int}
     */
    private $wrong_type = ["name" => 123, "age" => "wrong"]; // Wrong types

    /**
     * @var array{name: string, age: int}
     */
    private $missing_field = ["name" => "Charlie"]; // Missing 'age' field
}
