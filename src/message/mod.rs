pub(crate) use request::Request;

pub(crate) mod request;
pub(crate) mod codec;

#[cfg(test)]
mod request_test;
#[cfg(test)]
mod codec_test;

#[macro_export]
macro_rules! get {
    ($pos:expr, $subject:ident) => {
        $subject.get($pos).ok_or(Error::MalformedRequest)?
    };
}