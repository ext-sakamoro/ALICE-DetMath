//! `assert_function_pins`: a table of the resolved identifiers passes in a `const` item; a wrong entry, an
//! unknown name and an empty table panic (in a `const` item that stops the compile: the doc test of the
//! function holds a wrong pin as `compile_fail`).

use alice_det_math::{assert_function_pins, function_pin, FUNCTION_PINS};

const GOOD: &[(&str, [u8; 32])] = &[
    ("sin", function_pin("sin")),
    ("sqrt64", function_pin("sqrt64")),
];
const _: () = assert_function_pins(GOOD);

#[test]
fn every_function_of_the_table_passes() {
    assert!(!FUNCTION_PINS.is_empty());
    assert_function_pins(FUNCTION_PINS);
}

#[test]
#[should_panic(expected = "a function this crate calls moved")]
fn one_changed_bit_in_the_last_entry_panics() {
    let mut pin = function_pin("sqrt64");
    pin[31] ^= 1;
    assert_function_pins(&[("sin", function_pin("sin")), ("sqrt64", pin)]);
}

#[test]
#[should_panic(expected = "a function this crate calls moved")]
fn another_functions_pin_panics() {
    assert_function_pins(&[("sin", function_pin("cos"))]);
}

#[test]
#[should_panic(expected = "not a function of alice-det-math")]
fn an_unknown_name_panics() {
    assert_function_pins(&[("no_such_function", [0; 32])]);
}

#[test]
#[should_panic(expected = "this compares nothing")]
fn an_empty_table_panics() {
    assert_function_pins(&[]);
}
