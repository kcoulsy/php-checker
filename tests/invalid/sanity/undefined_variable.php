<?php

// php-checker-ignore: strict_typing/force_return_type,strict_typing/consistent_return

function divide(int $a, int $b): int
{
    if ($b === 0) {
        return 0;
    }

    return $a / $c;
}

echo divide(10, 2);

