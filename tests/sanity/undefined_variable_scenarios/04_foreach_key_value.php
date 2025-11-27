<?php
// php-checker-test: only-rules=sanity/undefined_variable

// Scenario: Foreach key and value variables should not be flagged as undefined
// Expected: No errors

function processAssocArray() {
    $data = ['name' => 'John', 'age' => 30];

    foreach ($data as $key => $value) {
        echo $key . ': ' . $value;
    }
}

function processPhones() {
    $phones = [
        ['type' => 'mobile', 'number' => '123'],
        ['type' => 'home', 'number' => '456']
    ];

    foreach ($phones as $index => $phone) {
        echo $index . ': ' . $phone['type'];
    }
}
