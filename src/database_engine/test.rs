use std::io::Cursor;

use crate::database_engine::{
    database_engine_errors::DatabaseEngineError,
    wal::{read_wal, write_wal, WalOperation},
};

#[test]
pub fn test_wal() -> Result<(), DatabaseEngineError> {
    let inputs = [("test_key", "test_value"), ("test_key2", "12345")];

    let mut cursor = Cursor::new(Vec::new());

    for (key, value) in inputs {
        let key = bincode::serialize(key).unwrap();
        let value = bincode::serialize(value).unwrap();

        write_wal(&mut cursor, WalOperation::Update, key, value)?;
    }

    cursor.set_position(0);
    let read = read_wal(&mut cursor)?;

    assert_eq!(read.len(), inputs.len());

    for (result, (key, value)) in read.iter().zip(inputs) {
        let deserialized_key: String = bincode::deserialize(&result.key).unwrap();
        let deserialized_value: String = bincode::deserialize(&result.value).unwrap();

        assert_eq!(&deserialized_key, key);
        assert_eq!(&deserialized_value, value);
    }

    Ok(())
}
