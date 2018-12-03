use typed_arena::Arena;

use cfg_if::cfg_if;
cfg_if! {
    if #[cfg(feature = "thread")] {
        #[allow(dead_code)]
        pub type Rc<T> = std::sync::Arc<T>;
    } else {
        #[allow(dead_code)]
        pub type Rc<T> = std::rc::Rc<T>;
    }
}

pub trait Intercalate<T> {
    fn intercalate(&self, sep: &T) -> Vec<T>;
}

pub trait JoinMany<T> {
    fn join_many(&self, sep: &[T]) -> Vec<T>;
}

impl<T: Clone> JoinMany<T> for [Vec<T>] {
    fn join_many(&self, sep: &[T]) -> Vec<T> {
        let mut iter = self.iter();
        let first = match iter.next() {
            Some(first) => first,
            None => return vec![],
        };
        let len = self.len();
        let mut result: Vec<T> = Vec::with_capacity(len + (len - 1) * sep.len());
        result.extend_from_slice(first);

        for v in iter {
            result.extend_from_slice(&sep);
            result.extend_from_slice(v);
        }
        result
    }
}

impl<T: Clone> Intercalate<T> for [T] {
    fn intercalate(&self, sep: &T) -> Vec<T> {
        let mut iter = self.iter();
        let first = match iter.next() {
            Some(first) => first,
            None => return vec![],
        };
        let mut result: Vec<T> = Vec::with_capacity(self.len() * 2 - 1);
        result.push(first.clone());

        for v in iter {
            result.push(sep.clone());
            result.push(v.clone())
        }
        result
    }
}

pub trait PartitionArenaErrors<O, E>: Iterator<Item = Result<O, E>>
where
    O: Sized,
    Self: Sized,
{
    fn partition_results<'a>(self) -> Result<Vec<O>, Vec<E>> {
        let mut errors = Vec::new();
        let oks = self
            .filter_map(|res| match res {
                Ok(ok) => Some(ok),
                Err(e) => {
                    errors.push(e);
                    None
                }
            })
            .collect();
        if errors.len() > 0 {
            Err(errors)
        } else {
            Ok(oks)
        }
    }

    fn partition_arena_results<'a>(self, arena: &'a Arena<O>) -> Result<&'a [O], Vec<E>> {
        let mut errors = Vec::new();
        let oks = self.filter_map(|res| match res {
            Ok(ok) => Some(ok),
            Err(e) => {
                errors.push(e);
                None
            }
        });
        let oks = arena.alloc_extend(oks);
        if errors.len() > 0 {
            Err(errors)
        } else {
            Ok(oks)
        }
    }
}

impl<O, E, I: Iterator<Item = Result<O, E>>> PartitionArenaErrors<O, E> for I {}