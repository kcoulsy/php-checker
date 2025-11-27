<?php

class Test {
    /**
     * @return array<string, int>
     */
    public function getMap() {
        return ["key1" => 123, "key2" => 456];
    }

    /**
     * @return array<int, string>
     */
    public function getNames() {
        return [0 => "Alice", 1 => "Bob"];
    }
}
