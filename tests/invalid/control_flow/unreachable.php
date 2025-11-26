<?php

function alwaysReturnEarly(): void
{
    return;
    echo "this line is unreachable";
}

alwaysReturnEarly();

