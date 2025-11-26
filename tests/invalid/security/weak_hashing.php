<?php

// php-checker-ignore: sanity/undefined_variable,cleanup/unused_variable,security/hard_coded_keys

// Weak hashing used for passwords - should trigger warning
$password = md5("secret");

// Weak hashing assigned to password variable - should trigger warning
$userPassword = sha1($input);

// Weak hashing in password context - should trigger warning
$hashedPassword = md5($userInput);

// OK - md5 used for non-password purposes
$checksum = md5($fileContent);

// OK - sha1 used for non-password purposes
$fileHash = sha1($fileData);

// OK - using secure password hashing
$secureHash = password_hash("secret", PASSWORD_DEFAULT);

// OK - md5 with password in variable name but not used for hashing
$somePassword = "secret";
$hash = hash('sha256', $somePassword);

$passwordHash = md5("test"); // Should trigger warning
$passwd = sha1("test"); // Should trigger warning

// OK - not password related
$dataHash = md5("data");
$contentSha1 = sha1("content");
