<?php

function sum(int ...$values): int
{
    return array_reduce(
        $values,
        fn(?int $carry, int $next): int => ($carry ?? 0) + $next,
        0
    );
}

$total = sum(1, 2, 3, 4);
echo "sum: $total\n";

