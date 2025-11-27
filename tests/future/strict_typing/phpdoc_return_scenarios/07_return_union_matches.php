<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_check

// Scenario: @return union type matches native union type hint
// Expected: No errors

class TestReturnUnionMatches {
    /**
     * @return int|string
     */
    function returnsIntOrString(): int|string {
        // OK - PHPDoc matches native type
        return 42;
    }

    /**
     * @return int|string|bool
     */
    function returnsMultipleTypes(): int|string|bool {
        // OK - PHPDoc matches native union type
        return true;
    }

    /**
     * @return User|Admin
     */
    function returnsUserOrAdmin(): User|Admin {
        // OK - union of objects
        return new User();
    }
}
