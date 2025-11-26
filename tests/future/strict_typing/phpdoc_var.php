<?php

// Scenario 1: ✓ Property with correct @var type
class CorrectProperty {
    /**
     * @var string
     */
    private $name = "test";
}

// Scenario 2: ✗ Property assigned wrong type vs @var
class WrongPropertyType {
    /**
     * @var string
     */
    private $name = 123;  // Error: assigning int to string property
}

// Scenario 3: ✓ Inline @var type casting in assignment
function inlineVarCast() {
    /** @var \DateTime $date */
    $date = createDate();
    $date->format('Y-m-d');
}

// Scenario 4: ✗ Inline @var claims wrong type
function wrongInlineVar() {
    /** @var string $value */
    $value = 123;  // Error: assigning int to string variable
    echo $value;
}

// Scenario 5: ✗ Variable reassigned to incompatible type after @var
function reassignmentAfterVar() {
    /** @var string $text */
    $text = "hello";
    $text = 456;  // Error: reassigning int to string variable
    echo $text;
}

// Scenario 6: ✓ @var with generic array type
class UserCollection {
    /**
     * @var User[]
     */
    private $users = [];
}

// Scenario 7: ✗ Array element wrong type vs @var User[]
function wrongArrayElementType() {
    /** @var User[] $users */
    $users = [1, 2, 3];  // Error: array contains int, expected User[]
    return $users;
}

// Scenario 8: ✓ @var with associative array
class Configuration {
    /**
     * @var array<string, int>
     */
    private $settings = [];
}

// Scenario 9: ✗ Wrong key/value type in associative array
function wrongAssocArrayType() {
    /** @var array<string, int> $scores */
    $scores = [1 => "wrong"];  // Error: int key and string value, expected string key and int value
    return $scores;
}

// Additional: @var on multiple properties
class MultipleProperties {
    /**
     * @var int
     */
    private $id;

    /**
     * @var string
     */
    private $name;

    /**
     * @var bool
     */
    private $active = "yes";  // Error: string assigned to bool property
}

// Additional: @var with union types
function unionTypeVar() {
    /** @var int|string $value */
    $value = 42;
    $value = "test";  // OK: both int and string are in union
    $value = true;  // Error: bool not in int|string union
}

// Additional: @var with nullable type
function nullableVar() {
    /** @var ?string $optional */
    $optional = null;
    $optional = "test";
    $optional = 123;  // Error: int not compatible with ?string
}

// Additional: Property type changes in method
class PropertyTypeChange {
    /**
     * @var string
     */
    private $value;

    public function __construct() {
        $this->value = "initial";
    }

    public function setValue() {
        $this->value = 999;  // Error: assigning int to string property
    }
}

// Additional: Static property with @var
class StaticProperty {
    /**
     * @var int
     */
    private static $counter = 0;

    public static function increment() {
        self::$counter = "wrong";  // Error: assigning string to int property
    }
}

// Additional: Constant value type checking
class ConstantType {
    /**
     * @var int
     */
    public const MAX_SIZE = "100";  // Error: string assigned to int constant
}

// Additional: Complex nested array type
function nestedArrayVar() {
    /** @var array<string, array<int, User>> $userGroups */
    $userGroups = [
        "admin" => [new User()],
        "guest" => [1, 2, 3]  // Error: int[] instead of User[]
    ];
}

// Additional: @var on loop variable
function loopVariable() {
    /** @var int[] $numbers */
    $numbers = [1, 2, 3];

    foreach ($numbers as $num) {
        /** @var int $num */
        echo $num;
    }

    /** @var string[] $strings */
    $strings = [1, 2, 3];  // Error: int[] assigned to string[] variable
}

// Additional: Type narrowing with @var
function typeNarrowing($mixed) {
    /** @var string $mixed */
    // After this point, $mixed should be treated as string
    $length = strlen($mixed);

    // But if we reassign with different type:
    $mixed = 123;  // Error: int assigned to string variable
}

// Additional: @var on class property with initialization
class PropertyInit {
    /**
     * @var \DateTime
     */
    private $created;

    public function __construct() {
        $this->created = new \DateTime();  // OK
    }

    public function reset() {
        $this->created = "2024-01-01";  // Error: string assigned to DateTime property
    }
}

// Additional: Array destructuring with @var
function arrayDestructuring() {
    /** @var array{id: int, name: string} $data */
    $data = ["id" => 1, "name" => "test"];

    /** @var array{id: int, name: string} $wrong */
    $wrong = ["id" => "one", "name" => 123];  // Error: id should be int, name should be string
}

// Additional: Reference variable with @var
function referenceVar() {
    /** @var int $original */
    $original = 10;

    $reference = &$original;
    $reference = "string";  // Error: assigning string to int variable (via reference)
}

// Additional: Global variable with @var
/** @var string $globalString */
$globalString = 123;  // Error: int assigned to string variable

// Additional: Superglobal annotation
function superglobalAnnotation() {
    /** @var array<string, string> $_POST */
    // Trying to narrow superglobal type
    $value = $_POST['key'];
}
