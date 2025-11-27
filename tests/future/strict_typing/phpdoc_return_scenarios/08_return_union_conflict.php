<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_check

// Scenario: @return union type conflicts with native union type hint
// Expected: Errors on lines 9, 17, 25

class TestReturnUnionConflict {
    /**
     * @return int|string
     */
    function wrongUnion(): int|bool {
        // Error - bool is not string
        return true;
    }

    /**
     * @return int|string
     */
    function differentUnion(): string|float {
        // Error - types don't match
        return 1.5;
    }

    /**
     * @return User|Admin
     */
    function wrongObjectUnion(): User|Guest {
        // Error - Guest is not Admin
        return new Guest();
    }
}
