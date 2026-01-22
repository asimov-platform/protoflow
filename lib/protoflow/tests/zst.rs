// This is free and unencumbered software released into the public domain.

use protoflow::{
    blocks::Const, runtimes::StdRuntime, transports::MpscTransport, System, SystemExecution,
};

#[test]
fn const_with_numeric_zero() -> Result<(), ()> {
    let transport = MpscTransport::new();
    let runtime = StdRuntime::new(transport).unwrap();

    let mut system = System::new(&runtime);
    let constant: Const<i32> = system.block(Const {
        output: system.output(),
        value: 0,
    });
    let output = system.input();

    system.connect(&constant.output, &output);

    let process = SystemExecution::execute(system).unwrap();

    assert_eq!(output.recv(), Ok(Some(0))); // not Ok(None)
    process.join().unwrap();
    Ok(())
}
