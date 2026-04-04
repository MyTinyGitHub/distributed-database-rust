use std::error::Error;

use tonic::{Request, Response};

use crate::storage::{
    storage_engine_service_client::StorageEngineServiceClient, CreateTableRequest,
    DropIndexRequest, IndexKey, ReadByIndexRequest, RegisterIndexRequest, WriteRequest,
};

pub mod wal {
    tonic::include_proto!("wal");
}

pub mod storage {
    tonic::include_proto!("storage");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "http://[::1]:50051";
    let eng_addr = "http://[::1]:50052";
    // let mut client = WalServiceClient::connect(addr).await?;
    let mut client = StorageEngineServiceClient::connect(eng_addr).await?;

    // let write_request = WriteRequest {
    //     partition_name: "abc".to_owned(),
    //     operation: 1,
    //     key: bincode::serialize("test_key")?,
    //     value: bincode::serialize("test_value")?,
    // };
    let table_name = "t".to_owned();
    let index_name = "i".to_owned();
    let index_key = "k".as_bytes();
    let data = bincode::serialize("v").unwrap();

    let engine_request = CreateTableRequest {
        table: table_name.clone(),
    };

    let create_table_request = Request::new(engine_request);
    let _ = client.create_table(create_table_request).await?;

    let register_index_request = RegisterIndexRequest {
        table: table_name.clone(),
        index_name: index_name.clone(),
    };

    let _ = client
        .register_index(Request::new(register_index_request))
        .await?;

    let index_key_struct = IndexKey {
        index_name: index_name.clone(),
        key: index_key.to_vec(),
    };

    let mut index_keys = Vec::new();
    index_keys.push(index_key_struct);

    let insert_data = WriteRequest {
        table: table_name.clone(),
        row_data: data,
        index_keys: index_keys,
    };

    let _ = client.write(Request::new(insert_data)).await?;

    let read_by_index = ReadByIndexRequest {
        table: table_name.clone(),
        index_name: index_name.clone(),
        key: index_key.to_vec(),
        transaction_id: 10,
    };

    let response = client.read_by_index(Request::new(read_by_index)).await?;
    let response = response.get_ref();

    for data in &response.data {
        let response_deserialized: &str = bincode::deserialize(data).unwrap();
        println!("{:?}", response_deserialized);
    }

    let drop_non_existent_index = DropIndexRequest {
        table: table_name,
        index_name: "test_non_existent".to_owned(),
    };

    let response = client
        .drop_index(Request::new(drop_non_existent_index))
        .await;

    assert!(response.is_err());
    println!("{:?}", response.err());

    // let request = Request::new(write_request);
    // let _ = client.write(request).await?;

    // let response = client
    //     .read(ReadRequest {
    //         partition_name: "abc".to_string(),
    //     })
    //     .await?;

    // response.get_ref().entries.iter().for_each(|r| {
    //     let key: &str = bincode::deserialize(&r.key).unwrap();
    //     let value: &str = bincode::deserialize(&r.value).unwrap();
    //     let op: &str = r.operation().as_str_name();

    //     println!("operation: {}, key: {}, value: {}", op, key, value);
    // });

    Ok(())
}
