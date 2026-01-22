// This is free and unencumbered software released into the public domain.

use bytes::Bytes;
use protoflow_blocks::{Const, IoBlocks, System, SystemBuilding, SystemExecution};
use protoflow_core::{runtimes::StdRuntime, transports::MpscTransport};

#[test]
fn json_roundtrip() -> Result<(), ()> {
    let mut system = System::new(&StdRuntime::new(MpscTransport::new()).unwrap());

    let input_bytes = Bytes::from(r#"[null,true,1,10.1,"hello!",{"1":false,"2":[1,2]}]"#);

    let input = system.block(Const::with_system(&system, input_bytes.clone()));
    let decode = system.decode_json();
    let encode = system.encode_json();
    let output = system.input();

    system.connect(&input.output, &decode.input);
    system.connect(&decode.output, &encode.input);
    system.connect(&encode.output, &output);

    let process = system.execute().unwrap();

    let message = output.recv().unwrap().unwrap();

    process.join().unwrap();

    assert_eq!(input_bytes, message);

    Ok(())
}
