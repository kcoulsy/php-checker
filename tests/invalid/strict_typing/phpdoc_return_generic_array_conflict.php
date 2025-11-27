<?php

class Test {
    /**
     * @return array<string, int>
     */
    public function getMap() {
        return [
            "key1" => 123,      // OK
            999 => 456,         // Wrong key type: int instead of string
            "key2" => "wrong"   // Wrong value type: string instead of int
        ];
    }
}
