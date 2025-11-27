<?php

class Test {
    /**
     * @var array<string, int>
     */
    private $map = [
        "key1" => 123,      // OK
        999 => 456,         // Wrong key type: int instead of string
        "key2" => "wrong"   // Wrong value type: string instead of int
    ];
}
