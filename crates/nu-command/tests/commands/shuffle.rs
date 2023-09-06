use nu_test_support::{nu};

#[test]
fn shuffle_basic_list() {
    let actual = nu!("[1 2 3 4 5] | shuffle");

    assert!(actual.out.contains('1'));
    assert!(actual.out.contains('2'));
    assert!(actual.out.contains('3'));
    assert!(actual.out.contains('4'));
    assert!(actual.out.contains('5'));
}

#[test]
fn shuffle_length_list() {
    let actual = nu!("[1 2 3 4 5] | shuffle | length");

    assert_eq!(actual.out, "5");
}

#[test]
fn shuffle_table() {
    let actual = nu!(
        r#"
        let table = [[name age]; ['alice' 51] ['bob' 55] ['eve' 33]];
        $table | shuffle | get name | str join " "
        "#
    );

    assert!(actual.out.contains("alice"));
    assert!(actual.out.contains("bob"));
    assert!(actual.out.contains("eve"));
}

#[test]
fn shuffle_record() {
    let actual = nu!("{alice: 51 bob: 55 eve: 33} | shuffle");

    assert!(actual.out.contains("alice"));
    assert!(actual.out.contains("bob"));
    assert!(actual.out.contains("eve"));
}

#[test]
fn shuffle_record_length() {
    let actual = nu!("{alice: 51 bob: 55 eve: 33} | shuffle | columns | length");

    assert_eq!(actual.out, "3");
}
