use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct Message {
    r#type: MessageType,
    message: String,
}

#[macro_export]
macro_rules! info {
    ($fstring:literal) => {
      crate::docker::info!($fstring, )
    };
    ($fstring:literal, $($a:expr),*) => {
        println!(
          "{}",
          serde_json::to_string(
            &crate::docker::Message::info(
              format!(
                $fstring,
                $( $a ),*
              )
            )
          ).unwrap()
        )
    };
}

#[macro_export]
macro_rules! error {
    ($fstring:literal) => {
      crate::docker::error!($fstring, )
    };
    ($fstring:literal, $($a:expr)*) => {
        println!(
          "{}",
          serde_json::to_string(
            &crate::docker::Message::error(
              format!(
                $fstring,
                $( $a ),*
              )
            )
          ).unwrap()
        )
    };
}

pub(crate) use info;
pub(crate) use error;

impl Message {
    pub(crate) fn info<S: Into<String>>(msg: S) -> Self {
        Self {
            r#type: MessageType::Info,
            message: msg.into(),
        }
    }

    pub(crate) fn error<S: Into<String>>(msg: S) -> Self {
        Self {
            r#type: MessageType::Error,
            message: msg.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum MessageType {
    Debug,
    Info,
    Error,
    Setenv,
}
