<?php

class Test {
    /**
     * @var array<int, array{name: string, age: int}>
     */
    private $users = [
        "wrong" => ["name" => "Alice", "age" => 30],  // Key should be int, not string
        1 => ["name" => 123, "age" => "wrong"]         // name should be string, age should be int
    ];
}
