// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

//! Transerialization extends the set of serde-compatible types (for given de/serializers).
//! By "hackishly" transmuting references across serde boundaries as u64s.
//! Type-safety is enforced using special struct names for each "magic type".
//! Memory-safety relies on transerialized values being "pinned" during de/serialization.

pub(crate) const MAGIC_FIELD: &str = "$__v8_magic_field";

pub(crate) trait MagicType {
    const NAME: &'static str;
    const MAGIC_NAME: &'static str;
}

pub(crate) fn magic_serialize<T, S>(serializer: S, x: &T) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: MagicType,
{
    use serde::ser::SerializeStruct;

    // SERIALIZATION CRIMES
    let mut s = serializer.serialize_struct(T::MAGIC_NAME, 1)?;
    let ptr = x as *const T as u64;
    s.serialize_field(MAGIC_FIELD, &ptr)?;
    s.end()
}

pub(crate) fn magic_deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: MagicType,
{
    struct ValueVisitor<T> {
        p1: std::marker::PhantomData<T>,
    }

    impl<'de, T: MagicType> serde::de::Visitor<'de> for ValueVisitor<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a ")?;
            formatter.write_str(T::NAME)
        }

        fn visit_u64<E>(self, ptr: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // SAFETY: opaque ptr originates from visit_magic, which forgets ownership so we can take it
            Ok(unsafe {
                {
                    // DESERIALIZATION CRIMES

                    // serde_v8 was originally taking a pointer to a stack value here. This code is crazy
                    // but there's no way to fix it easily. As a bandaid, we boxed it before.
                    let ptr: *mut T = ptr as _;
                    *Box::<T>::from_raw(ptr)
                }
            })
        }
    }

    deserializer.deserialize_struct(
        T::MAGIC_NAME,
        &[MAGIC_FIELD],
        ValueVisitor::<T> {
            p1: std::marker::PhantomData,
        },
    )
}

macro_rules! impl_magic {
    ($t:ty) => {
        impl crate::transl8::MagicType for $t {
            const NAME: &'static str = stringify!($t);
            const MAGIC_NAME: &'static str = concat!("$__v8_magic_", stringify!($t));
        }

        impl serde::Serialize for $t {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                crate::transl8::magic_serialize(serializer, self)
            }
        }

        impl<'de> serde::Deserialize<'de> for $t {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                crate::transl8::magic_deserialize(deserializer)
            }
        }
    };
}
