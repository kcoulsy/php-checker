<?php

class User {
    /**
     * @var array{name: string, age: int}
     */
    private $correctPerson = ["name" => "Alice", "age" => 30]; // ✓ Correct

    /**
     * @var array{age: int, name: string}  
     */
    private $orderIndependent = ["name" => "Bob", "age" => 25]; // ✓ Order doesn't matter

    /**
     * @var array{name: string, age: int}
     */
    private $wrongTypes = ["name" => 123, "age" => "wrong"]; // ✗ Wrong types
}
