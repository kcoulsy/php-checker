<?php

// php-checker-ignore: strict_typing/force_return_type

function test_impossible_break(): void {
    $value = 1;
    switch ($value) {
        case 1:
            echo "one";
            break;
            // impossible break
            break;
        case 2:
            echo "two";
            return;
            // impossible break after return
            break;
        case 3:
            echo "three";
            return;
        default:
            return;
    }
}

function test_impossible_return(): void {
    $value = 1;
    switch ($value) {
        case 1:
            echo "one";
            return;
            // impossible return after return
            return;
        case 2:
            echo "two";
            return;
        default:
            return;
    }
}
