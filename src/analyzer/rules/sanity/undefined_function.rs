use super::DiagnosticRule;
use super::helpers::{child_by_kind, collect_function_signatures, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use std::collections::HashSet;

/// Common PHP built-in functions to avoid false positives.
/// This list covers the most frequently used functions from the PHP standard library.
static PHP_BUILTINS: &[&str] = &[
    // Array functions
    "array_change_key_case", "array_chunk", "array_combine", "array_count_values",
    "array_diff", "array_diff_assoc", "array_diff_key", "array_fill", "array_fill_keys",
    "array_filter", "array_flip", "array_intersect", "array_intersect_assoc",
    "array_intersect_key", "array_key_exists", "array_key_first", "array_key_last",
    "array_keys", "array_map", "array_merge", "array_merge_recursive", "array_multisort",
    "array_pad", "array_pop", "array_product", "array_push", "array_rand", "array_reduce",
    "array_replace", "array_replace_recursive", "array_reverse", "array_search",
    "array_shift", "array_slice", "array_splice", "array_sum", "array_unique",
    "array_unshift", "array_values", "array_walk", "array_walk_recursive",
    "arsort", "asort", "compact", "count", "extract", "in_array", "krsort", "ksort",
    "list", "natcasesort", "natsort", "range", "rsort", "shuffle", "sizeof", "sort",
    "uasort", "uksort", "usort",
    // String functions
    "addcslashes", "addslashes", "bin2hex", "chop", "chr", "chunk_split",
    "convert_uuencode", "convert_uudecode", "count_chars", "crc32", "crypt",
    "explode", "fprintf", "get_html_translation_table", "hebrev", "hex2bin",
    "html_entity_decode", "htmlentities", "htmlspecialchars", "htmlspecialchars_decode",
    "implode", "join", "lcfirst", "levenshtein", "ltrim", "md5", "money_format",
    "nl2br", "number_format", "ord", "quoted_printable_decode", "quoted_printable_encode",
    "quotemeta", "rtrim", "sha1", "similar_text", "soundex", "sprintf", "sscanf",
    "str_contains", "str_ends_with", "str_ireplace", "str_pad", "str_repeat",
    "str_replace", "str_rot13", "str_split", "str_starts_with", "str_word_count",
    "strcasecmp", "strchr", "strcmp", "strcoll", "strcspn", "strip_tags", "stripcslashes",
    "stripos", "stripslashes", "stristr", "strlen", "strnatcasecmp", "strnatcmp",
    "strncasecmp", "strncmp", "strpbrk", "strpos", "strrchr", "strrev", "strripos",
    "strrpos", "strstr", "strtolower", "strtoupper", "strtr", "substr", "substr_compare",
    "substr_count", "substr_replace", "trim", "ucfirst", "ucwords", "vprintf", "vsprintf",
    "wordwrap", "mb_strlen", "mb_substr", "mb_strtolower", "mb_strtoupper", "mb_strpos",
    "mb_detect_encoding", "mb_convert_encoding", "mb_internal_encoding",
    // Math functions
    "abs", "acos", "acosh", "asin", "asinh", "atan", "atan2", "atanh", "base_convert",
    "bindec", "ceil", "cos", "cosh", "decbin", "dechex", "decoct", "deg2rad", "exp",
    "expm1", "fdiv", "floor", "fmod", "hexdec", "hypot", "intdiv", "is_finite",
    "is_infinite", "is_nan", "log", "log10", "log1p", "max", "min", "octdec", "pi",
    "pow", "rad2deg", "rand", "round", "sin", "sinh", "sqrt", "srand", "tan", "tanh",
    "mt_rand", "mt_srand", "mt_getrandmax", "random_int", "random_bytes",
    // Type functions
    "boolval", "doubleval", "empty", "floatval", "get_debug_type", "gettype", "intval",
    "is_array", "is_bool", "is_callable", "is_countable", "is_double", "is_finite",
    "is_float", "is_infinite", "is_int", "is_integer", "is_iterable", "is_long",
    "is_nan", "is_null", "is_numeric", "is_object", "is_real", "is_resource",
    "is_string", "isset", "unset", "print_r", "serialize", "settype", "strval",
    "unserialize", "var_dump", "var_export",
    // Date/time functions
    "checkdate", "date", "date_create", "date_format", "date_parse", "date_parse_from_format",
    "date_diff", "date_add", "date_sub", "date_interval_create_from_date_string",
    "getdate", "gettimeofday", "gmdate", "gmmktime", "gmstrftime", "localtime",
    "microtime", "mktime", "strftime", "strptime", "strtotime", "time",
    // File functions
    "basename", "chdir", "chgrp", "chmod", "chown", "clearstatcache", "copy",
    "delete", "dirname", "disk_free_space", "disk_total_space", "diskfreespace",
    "fclose", "feof", "fflush", "fgetc", "fgetcsv", "fgets", "fgetss", "file",
    "file_exists", "file_get_contents", "file_put_contents", "fileatime", "filectime",
    "filegroup", "fileinode", "filemtime", "fileowner", "fileperms", "filesize",
    "filetype", "flock", "fnmatch", "fopen", "fpassthru", "fputcsv", "fputs",
    "fread", "fscanf", "fseek", "fstat", "ftell", "ftruncate", "fwrite",
    "glob", "is_dir", "is_executable", "is_file", "is_link", "is_readable",
    "is_uploaded_file", "is_writable", "is_writeable", "link", "linkinfo",
    "lstat", "mkdir", "move_uploaded_file", "parse_ini_file", "parse_ini_string",
    "pathinfo", "pclose", "popen", "readdir", "readfile", "readlink", "realpath",
    "rename", "rewind", "rmdir", "scandir", "stat", "symlink", "tempnam",
    "tmpfile", "touch", "umask", "unlink",
    // Class/object functions
    "class_alias", "class_exists", "class_implements", "class_parents", "class_uses",
    "get_class", "get_object_vars", "get_parent_class", "interface_exists",
    "is_a", "is_subclass_of", "method_exists", "property_exists", "trait_exists",
    // JSON functions
    "json_decode", "json_encode", "json_last_error", "json_last_error_msg",
    // Misc functions
    "call_user_func", "call_user_func_array", "chr", "chunk_split", "compact",
    "defined", "die", "echo", "eval", "exit", "extract", "func_get_arg",
    "func_get_args", "func_num_args", "function_exists", "header", "headers_sent",
    "http_response_code", "ignore_user_abort", "list", "ob_end_clean",
    "ob_end_flush", "ob_get_clean", "ob_get_contents", "ob_get_flush", "ob_get_length",
    "ob_get_level", "ob_get_status", "ob_start", "print", "printf", "preg_grep",
    "preg_match", "preg_match_all", "preg_quote", "preg_replace", "preg_replace_callback",
    "preg_split", "set_error_handler", "set_exception_handler", "set_time_limit",
    "sleep", "usleep", "str_contains", "str_ends_with", "str_starts_with",
    "array_is_list", "str_pad", "array_any", "array_all",
    // Error/exception
    "error_log", "error_reporting", "trigger_error", "set_error_handler",
    "restore_error_handler", "debug_backtrace", "debug_print_backtrace",
    // Sorting
    "sort", "rsort", "asort", "arsort", "ksort", "krsort", "usort", "uasort", "uksort",
    // Hash/crypto
    "hash", "hash_algos", "hash_equals", "hash_file", "hash_hmac", "hash_pbkdf2",
    "md5", "sha1", "crc32", "openssl_random_pseudo_bytes",
    // URL functions
    "base64_decode", "base64_encode", "http_build_query", "parse_str", "parse_url",
    "rawurldecode", "rawurlencode", "urldecode", "urlencode",
    // Output buffering
    "ob_start", "ob_end_clean", "ob_end_flush", "ob_get_clean", "ob_get_contents",
    // Session
    "session_start", "session_destroy", "session_id", "session_name",
    "session_regenerate_id", "session_status", "session_unset",
    // PCRE
    "preg_match", "preg_match_all", "preg_replace", "preg_split", "preg_quote",
    // Reflection
    "get_class", "get_object_vars", "get_parent_class", "get_declared_classes",
    // Miscellaneous
    "compact", "extract", "list", "define", "constant", "defined",
    "microtime", "memory_get_usage", "memory_get_peak_usage",
    "phpversion", "php_uname", "php_sapi_name", "phpinfo",
    "ini_get", "ini_set", "ini_restore", "get_cfg_var",
    "extension_loaded", "get_loaded_extensions",
    "register_shutdown_function", "register_tick_function",
    "assert", "assert_options",
    "lcg_value", "getenv", "putenv",
    "sprintf", "printf", "fprintf", "vprintf", "vsprintf", "sscanf",
    "number_format",
    "nl2br", "strip_tags", "htmlspecialchars", "htmlentities",
    "setcookie", "setrawcookie", "header", "headers_list", "headers_sent",
    "ignore_user_abort", "connection_aborted", "connection_status",
    "sleep", "usleep", "time_nanosleep", "time_sleep_until",
    "pack", "unpack", "hex2bin", "bin2hex",
    "base_convert", "number_format",
    "array_column", "array_combine", "array_count_values", "array_fill",
    "array_fill_keys", "compact", "extract",
    "intl_get_error_code", "intl_get_error_message",
    // Casting-like functions
    "intval", "floatval", "strval", "boolval", "doubleval",
];

pub struct UndefinedFunctionRule;

impl UndefinedFunctionRule {
    pub fn new() -> Self {
        Self
    }

    fn builtins() -> HashSet<&'static str> {
        PHP_BUILTINS.iter().copied().collect()
    }
}

impl DiagnosticRule for UndefinedFunctionRule {
    fn name(&self) -> &str {
        "sanity/undefined_function"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let builtins = Self::builtins();
        let local_signatures = collect_function_signatures(parsed);
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_call_expression" {
                return;
            }

            // Only handle simple name calls (not $var() or ClassName::method())
            let name_node = match child_by_kind(node, "name") {
                Some(n) => n,
                None => return,
            };

            let name = match node_text(name_node, parsed) {
                Some(n) => n,
                None => return,
            };

            // Skip namespaced calls (too complex to resolve without full namespace info)
            if name.contains('\\') {
                return;
            }

            // Skip if it's a built-in
            if builtins.contains(name.as_str()) {
                return;
            }

            // Skip if defined locally
            if local_signatures.contains_key(&name) {
                return;
            }

            // Try to resolve via project context
            if context.resolve_function_symbol(&name, parsed).is_some() {
                return;
            }

            let start = name_node.start_position();
            diagnostics.push(diagnostic_for_node(
                parsed,
                name_node,
                Severity::Warning,
                format!(
                    "call to undefined function '{name}' at {}:{}",
                    start.row + 1,
                    start.column + 1
                ),
            ));
        });

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{
        assert_diagnostics_exact, assert_no_diagnostics, run_rule_with_context,
    };

    #[test]
    fn test_undefined_function_flagged() {
        let source = r#"<?php
myUndefinedFunction();
"#;
        let rule = UndefinedFunctionRule::new();
        let diagnostics = run_rule_with_context(&rule, source);
        assert_diagnostics_exact(
            &diagnostics,
            &["warning: call to undefined function 'myUndefinedFunction' at 2:1"],
        );
    }

    #[test]
    fn test_locally_defined_function_ok() {
        let source = r#"<?php
function myFunc(): void {}
myFunc();
"#;
        let rule = UndefinedFunctionRule::new();
        let diagnostics = run_rule_with_context(&rule, source);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_php_builtins_ok() {
        let source = r#"<?php
$arr = [1, 2, 3];
$len = count($arr);
$str = implode(", ", $arr);
$upper = strtoupper("hello");
"#;
        let rule = UndefinedFunctionRule::new();
        let diagnostics = run_rule_with_context(&rule, source);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_array_functions_ok() {
        let source = r#"<?php
$arr = [3, 1, 2];
sort($arr);
$mapped = array_map(fn($x) => $x * 2, $arr);
$filtered = array_filter($arr, fn($x) => $x > 1);
"#;
        let rule = UndefinedFunctionRule::new();
        let diagnostics = run_rule_with_context(&rule, source);
        assert_no_diagnostics(&diagnostics);
    }
}
