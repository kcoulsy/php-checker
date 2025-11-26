<?php
// Scenario: Property assigned wrong type vs @var
// Expected: Error on line 8 (string property with int value)

class WrongPropertyType {
    /**
     * @var string
     */
    private $name = 123;  // Error: int assigned to string property
}
