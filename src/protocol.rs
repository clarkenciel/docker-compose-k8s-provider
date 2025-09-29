pub(crate) enum Request {
    Health,
    Stop,
}

impl Request {
    pub(crate) fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Health => vec![RECORD_SEPARATOR, REQUEST_TAG, 1, RECORD_SEPARATOR],
            Self::Stop => vec![RECORD_SEPARATOR, REQUEST_TAG, 2, RECORD_SEPARATOR],
        }
    }

    pub(crate) fn from_reader<R: std::io::Read>(r: R) -> Result<Self, RequestParseError> {
        RequestParser(r).try_into()
    }
}

pub(crate) struct RequestParser<R>(R);

impl<R: std::io::Read> TryFrom<RequestParser<R>> for Request {
    type Error = RequestParseError;

    fn try_from(mut value: RequestParser<R>) -> Result<Self, Self::Error> {
        let mut bytes = [0x0u8; 4];
        value
            .0
            .read_exact(&mut bytes)
            .map_err(RequestParseError::Eof)?;

        match bytes {
            [RECORD_SEPARATOR, REQUEST_TAG, 1, RECORD_SEPARATOR] => Ok(Self::Health),
            [RECORD_SEPARATOR, REQUEST_TAG, 2, RECORD_SEPARATOR] => Ok(Self::Stop),
            r => Err(RequestParseError::Unknown(r)),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum RequestParseError {
    #[error("Unexpected end of input")]
    Eof(std::io::Error),

    #[error("Unknown request")]
    Unknown([u8; 4]),
}

pub(crate) enum Response {
    Ok,
    Err,
}

impl Response {
    pub(crate) fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Ok => vec![RECORD_SEPARATOR, RESPONSE_TAG, 1, RECORD_SEPARATOR],
            Self::Err => vec![RECORD_SEPARATOR, RESPONSE_TAG, 2, RECORD_SEPARATOR],
        }
    }

    pub(crate) fn from_reader<R: std::io::Read>(r: R) -> Result<Self, ResponseParseError> {
        ResponseParser(r).try_into()
    }
}

pub(crate) struct ResponseParser<R>(R);

impl<R: std::io::Read> TryFrom<ResponseParser<R>> for Response {
    type Error = ResponseParseError;

    fn try_from(mut value: ResponseParser<R>) -> Result<Self, Self::Error> {
        let mut bytes = [0x0u8; 4];
        value
            .0
            .read_exact(&mut bytes)
            .map_err(ResponseParseError::Eof)?;

        match bytes {
            [RECORD_SEPARATOR, RESPONSE_TAG, 1, RECORD_SEPARATOR] => Ok(Self::Ok),
            [RECORD_SEPARATOR, RESPONSE_TAG, 2, RECORD_SEPARATOR] => Ok(Self::Err),
            r => Err(ResponseParseError::Unknown(r)),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub(crate) enum ResponseParseError {
    #[error("Unexpected end of input: {0}")]
    Eof(std::io::Error),

    #[error("Unknown response")]
    Unknown([u8; 4]),
}

const RECORD_SEPARATOR: u8 = 0x1e;
const REQUEST_TAG: u8 = 1;
const RESPONSE_TAG: u8 = 1;
