<?php
// php-checker-test: only-rules=sanity/undefined_variable

// Scenario: Exception variables in catch clauses should not be flagged as undefined
// Expected: No errors

function handleError() {
    try {
        throw new Exception("Error");
    } catch (Exception $e) {
        echo $e->getMessage();
    }
}

function multipleCatch() {
    try {
        // something
    } catch (RuntimeException $re) {
        echo $re->getMessage();
    } catch (Exception $ex) {
        echo $ex->getMessage();
    }
}
