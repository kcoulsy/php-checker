<?php
// Scenario: Inline @var type casting in assignment
// Expected: No errors - valid type narrowing

function createDate(): object {
    return new \DateTime();
}

function inlineVarCast() {
    /** @var \DateTime $date */
    $date = createDate();
    $date->format('Y-m-d');  // OK: $date is known to be DateTime
}
