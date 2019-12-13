use chrono::format::ParseError as TimeParseError;
use chrono::NaiveDateTime;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

pub struct ParseError {
    inner: Box<dyn std::error::Error>,
}

impl fmt::Debug for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "parse error: {}", self.inner)
    }
}

impl From<std::net::AddrParseError> for ParseError {
    fn from(error: std::net::AddrParseError) -> Self {
        Self {
            inner: Box::new(error),
        }
    }
}

impl From<std::num::ParseFloatError> for ParseError {
    fn from(error: std::num::ParseFloatError) -> Self {
        Self {
            inner: Box::new(error),
        }
    }
}

impl From<std::num::ParseIntError> for ParseError {
    fn from(error: std::num::ParseIntError) -> Self {
        Self {
            inner: Box::new(error),
        }
    }
}

impl From<std::str::Utf8Error> for ParseError {
    fn from(error: std::str::Utf8Error) -> Self {
        Self {
            inner: Box::new(error),
        }
    }
}

pub type Int64Parser = dyn Fn(&[u8]) -> Result<i64, ParseError> + Send + Sync;
pub type UInt32Parser = dyn Fn(&[u8]) -> Result<u32, ParseError> + Send + Sync;
pub type Float64Parser = dyn Fn(&[u8]) -> Result<f64, ParseError> + Send + Sync;
pub type DateTimeParser = dyn Fn(&[u8]) -> Result<NaiveDateTime, TimeParseError> + Send + Sync;

#[derive(Clone)]
pub enum FieldParser {
    Int64(Arc<Int64Parser>),
    UInt32(Arc<UInt32Parser>),
    Float64(Arc<Float64Parser>),
    Utf8,
    DateTime(Arc<DateTimeParser>),
    Dict,
}

impl FieldParser {
    pub fn int64() -> Self {
        Self::Int64(Arc::new(parse::<i64>))
    }

    pub fn uint32() -> Self {
        Self::UInt32(Arc::new(parse::<u32>))
    }

    pub fn float64() -> Self {
        Self::Float64(Arc::new(parse::<f64>))
    }

    pub fn uint32_with_parser<P>(parser: P) -> Self
    where
        P: Fn(&[u8]) -> Result<u32, ParseError> + Send + Sync + 'static,
    {
        Self::UInt32(Arc::new(parser))
    }

    pub fn new_datetime<P>(parser: P) -> Self
    where
        P: Fn(&[u8]) -> Result<NaiveDateTime, TimeParseError> + Send + Sync + 'static,
    {
        Self::DateTime(Arc::new(parser))
    }
}

impl<'a> fmt::Debug for FieldParser {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int64(_) => write!(f, "Int64"),
            Self::UInt32(_) => write!(f, "UInt32"),
            Self::Float64(_) => write!(f, "Float64"),
            Self::Utf8 => write!(f, "Utf8"),
            Self::DateTime(_) => write!(f, "DateTime"),
            Self::Dict => write!(f, "Dict"),
        }
    }
}

fn parse<T>(v: &[u8]) -> Result<T, ParseError>
where
    T: FromStr,
    <T as FromStr>::Err: Into<ParseError>,
{
    std::str::from_utf8(v)?.parse::<T>().map_err(Into::into)
}
