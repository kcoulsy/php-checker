<?php
// Scenario: Property with correct @var type
// Expected: No errors

class CorrectProperty {
    /**
     * @var string
     */
    private $name = "test";

    /**
     * @var int
     */
    private $age = 25;

    /**
     * @var bool
     */
    private $active = true;
}
