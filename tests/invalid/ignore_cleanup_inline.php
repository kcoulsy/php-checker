<?php
// php-checker-ignore: cleanup // reason for ignoring

use Multi\Service as Svc;
use Multi\Client;

function takesTwo(int $a, int $b): void
{
}

Svc\takesTwo(1);

