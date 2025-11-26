<?php
// Scenario: Variable reassigned to incompatible type after @var
// Expected: Error on line 7 (reassigning int to string variable)

function reassignmentAfterVar() {
    /** @var string $text */
    $text = "hello";
    $text = 456;  // Error: reassigning int to string variable
    echo $text;
}
