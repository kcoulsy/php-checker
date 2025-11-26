<?php
// Scenario: Inline @var claims wrong type
// Expected: Error on line 5 (assigning int to string variable)

function wrongInlineVar() {
    /** @var string $value */
    $value = 123;  // Error: assigning int to string variable
    echo $value;
}
