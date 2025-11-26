<?php

$value = 'alpha';

switch ($value) {
    case 'alpha':
        echo 'first alpha';
        break;
    case 'beta':
        echo 'beta';
        break;
    case 'alpha':
        echo 'duplicate alpha';
        break;
    case 1:
        break;
    case 1:
        break;
}

