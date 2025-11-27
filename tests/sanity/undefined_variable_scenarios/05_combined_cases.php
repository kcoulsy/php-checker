<?php
// php-checker-test: only-rules=sanity/undefined_variable

// Scenario: Combined test with class properties, catch, and foreach
// Expected: No errors

class FormContact {
    public $originalEmail = null;
    public $originalPhones = [];

    public function updateContact() {
        try {
            foreach ($this->originalPhones as $index => $phone) {
                $this->validatePhone($phone);
            }
        } catch (Exception $e) {
            error_log($e->getMessage());
        }
    }

    private function validatePhone($phone) {
        // validation logic
    }
}
