use battler_wamp_values::WampList;
use battler_wamprat_error::WampError;
use battler_wamprat_message::WampApplicationMessage;
use battler_wamprat_schema_proc_macro::WampSchema;
use thiserror::Error;

#[test]
fn derive() {
    #[derive(WampList)]
    struct OneNumber(u64);

    #[derive(WampList)]
    struct TwoNumbers(u64, u64);

    #[derive(WampApplicationMessage)]
    struct Input(#[arguments] TwoNumbers);

    #[derive(WampApplicationMessage)]
    struct Output(#[arguments] OneNumber);

    #[derive(Debug, Error, WampError)]
    enum DivideError {
        #[error("cannot divide by 0")]
        #[uri("com.battler.error.divide_by_zero")]
        DivideByZero,
    }

    #[derive(WampApplicationMessage)]
    struct Ping;

    #[derive(WampSchema)]
    #[realm("com.battler.realm")]
    enum Calculator {
        #[rpc(input = Input, output = Output, uri = "com.battler.add")]
        Add,
        #[rpc(input = Input, output = Output, uri = "com.battler.divide")]
        Divide,
        #[pubsub(event = Ping, uri = "com.battler.ping")]
        Ping,
    }
}
