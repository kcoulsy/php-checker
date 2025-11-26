<?php

function maybeString(bool $flag)
{
    if ($flag) {
        return 'ok';
    }

    // Missing return for the `false` branch.
}

maybeString(false);

