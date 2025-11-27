<?php
// php-checker-test: only-rules=sanity/undefined_variable

// Scenario: Foreach value variables should not be flagged as undefined
// Expected: No errors

function processArray() {
    $items = [1, 2, 3];

    foreach ($items as $item) {
        echo $item;
    }
}
