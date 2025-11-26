<?php
// Scenario: @var with generic array type
// Expected: No errors - correct User[] array

class User {}

class UserCollection {
    /**
     * @var User[]
     */
    private $users = [];

    public function addUser(User $user): void {
        $this->users[] = $user;
    }
}
