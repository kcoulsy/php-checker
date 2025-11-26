<?php

function global_this() {
    return $this;
}

class Example {
    public static function build() {
        return $this;
    }
}

