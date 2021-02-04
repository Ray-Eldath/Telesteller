pub(crate) use request::Request;
pub(crate) use response::ResponseFrame;

pub(crate) mod request;
pub(crate) mod response;
pub(crate) mod codec;

#[cfg(test)]
mod request_test;
#[cfg(test)]
mod codec_test;
#[cfg(test)]
mod response_test;

#[macro_export]
macro_rules! get {
    ($pos:expr, $subject:ident) => {
        $subject.get($pos).ok_or(super::request::Error::MalformedRequest)?
    };
}

#[macro_export]
macro_rules! pub_struct {
    ($name:ident { $($field:ident: $t:ty,)*} ) => {
        #[derive(Debug, PartialEq)]
        pub(crate) struct $name {
            $(pub(crate) $field: $t),*
        }
    }
}

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub enum Qos {
    FireAndForget = 0,
    AcknowledgedDeliver = 1,
    AssuredDelivery = 2,
}