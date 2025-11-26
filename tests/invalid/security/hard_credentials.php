<?php

// php-checker-ignore: security/hard_coded_keys

db_connect('super-secret-password');
call_api('token', 'my-api-key');

