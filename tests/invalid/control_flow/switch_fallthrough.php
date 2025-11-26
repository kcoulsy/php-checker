<?php

function test_fallthrough_without_comment() {
    $value = 1;
    switch ($value) {
        case 1:
            echo "one";
            // falls through to case 2 without comment
        case 2:
            echo "one or two";
            break;
        case 3:
            echo "three";
            break;
    }
}

function test_fallthrough_with_comment() {
    $value = 1;
    switch ($value) {
        case 1:
            echo "one";
            // php-checker-ignore control_flow/fallthrough
        case 2:
            echo "one or two";
            break;
    }
}

function test_fallthrough_with_break() {
    $value = 1;
    switch ($value) {
        case 1:
            echo "one";
            break;
        case 2:
            echo "two";
            break;
    }
}

function test_fallthrough_with_return(): void {
    $value = 1;
    switch ($value) {
        case 1:
            echo "one";
            return;
        case 2:
            echo "two";
            return;
        default:
            return;
    }
}
