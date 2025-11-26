<?php

$guard = true;
$value = 42;
$incremented = $guard ? $value + 1 : $value;

function greet(string $name): string
{
    return "Hello, $name";
}

echo greet("World") . " ($incremented)";

