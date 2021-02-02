pub(crate) trait BoolExt {
    fn if_so<V>(self, v: V) -> Option<V>;
    fn if_so_then<F, R>(self, f: F) -> Option<R> where F: FnMut() -> R;
}

impl BoolExt for bool {
    fn if_so<V>(self, v: V) -> Option<V> {
        if self { Some(v) } else { None }
    }

    fn if_so_then<F, R>(self, mut f: F) -> Option<R> where F: FnMut() -> R {
        if self { Some(f()) } else { None }
    }
}
