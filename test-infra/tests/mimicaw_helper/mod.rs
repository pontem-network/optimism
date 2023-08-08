use anyhow::anyhow;
use mimicaw::Outcome;
use tokio::task::JoinError;

type TestHandleResult = std::result::Result<anyhow::Result<()>, JoinError>;

pub(crate) trait TestHandleResultToOutcom {
    fn to_outcome(self) -> Outcome;
}

impl TestHandleResultToOutcom for TestHandleResult {
    fn to_outcome(self) -> Outcome {
        match self.map_err(|err| anyhow!("{err}")).and_then(|v| v) {
            Ok(_) => Outcome::passed(),
            Err(err) => Outcome::failed().error_message(err.to_string()),
        }
    }
}
