use crate::{errors, model};
use crate::model::inst;
use crate::errors::ULogError;

/// Trait for deriving accessors for logged data.
pub trait ULogAccessorFactory {
    type Accessor;

    /// Create an accessor from the subscription definition format.
    fn from_format(format: &crate::model::def::Format) -> Result<Self::Accessor, ULogError>;
}

pub trait ULogAccessor {
    type Output;

    fn get_data(&self, field: &inst::Format) -> Result<Self::Output, ULogError>;
}

/// FromField
///
/// Converts an inst::Field to the specified type, with runtime
/// error checking.
///
/// Type information from the ULOG file is not available at compile
/// time.  These helper enable the #[derive(ULogData)] proc macro to
/// convert the fields in the annotated struct with runtime type checking
/// against the inst::Field obtained from the parser.
///
pub trait FromField: Sized {
    fn from_field(field: &inst::Field) -> Result<Self, crate::errors::ULogError>;
}

// --- Scalars ---
macro_rules! impl_fromfield_scalar {
    ($ty:ty, $variant:ident) => {
        impl FromField for $ty {
            fn from_field(field: &inst::Field) -> Result<Self, ULogError> {
                match &field.value {
                    inst::FieldValue::$variant(v) => Ok(*v),
                    other => Err(ULogError::TypeMismatch(format!(
                        "Expected {} but got {:?}", stringify!($ty), other
                    ))),
                }
            }
        }
    };
}

// --- Vectors ---
macro_rules! impl_fromfield_array {
    ($ty:ty, $variant:ident) => {
        impl FromField for Vec<$ty> {
            fn from_field(field: &inst::Field) -> Result<Self, ULogError> {
                match &field.value {
                    inst::FieldValue::$variant(v) => Ok(v.clone()),
                    other => Err(ULogError::TypeMismatch(format!(
                        "Expected Vec<{}> but got {:?}", stringify!($ty), other
                    ))),
                }
            }
        }
    };
}

// --- Scalar impls ---
impl_fromfield_scalar!(u8, ScalarU8);
impl_fromfield_scalar!(u16, ScalarU16);
impl_fromfield_scalar!(u32, ScalarU32);
impl_fromfield_scalar!(u64, ScalarU64);
impl_fromfield_scalar!(i8, ScalarI8);
impl_fromfield_scalar!(i16, ScalarI16);
impl_fromfield_scalar!(i32, ScalarI32);
impl_fromfield_scalar!(i64, ScalarI64);
impl_fromfield_scalar!(f32, ScalarF32);
impl_fromfield_scalar!(f64, ScalarF64);
impl_fromfield_scalar!(bool, ScalarBool);
impl_fromfield_scalar!(char, ScalarChar);

// --- Vector impls ---
impl_fromfield_array!(u8, ArrayU8);
impl_fromfield_array!(u16, ArrayU16);
impl_fromfield_array!(u32, ArrayU32);
impl_fromfield_array!(u64, ArrayU64);
impl_fromfield_array!(i8, ArrayI8);
impl_fromfield_array!(i16, ArrayI16);
impl_fromfield_array!(i32, ArrayI32);
impl_fromfield_array!(i64, ArrayI64);
impl_fromfield_array!(f32, ArrayF32);
impl_fromfield_array!(f64, ArrayF64);
impl_fromfield_array!(bool, ArrayBool);
impl_fromfield_array!(char, ArrayChar);


impl<T> FromField for Vec<T>
where
    T: ULogAccessorFactory,
    T::Accessor: ULogAccessor<Output = T>,
{
    fn from_field(
        field: &inst::Field,
    ) -> Result<Self, ULogError> {
        match &field.value {
            inst::FieldValue::ArrayOther(formats) => {
                if formats.is_empty() {
                    return Ok(::std::vec::Vec::new());
                }
                let mut results = ::std::vec::Vec::with_capacity(formats.len());
                let accessor = T::from_format(&formats[0].def_format, )?;
                for fmt in formats {
                    results.push(accessor.get_data(fmt)?);
                }
                Ok(results)
            }
            _ => {
                Err(
                    ULogError::TypeMismatch(
                        format!("expected array for field {}", field.name).into(),
                    ),
                )
            }
        }
    }
}

impl<T> FromField for T
where
    T: ULogAccessorFactory,
    T::Accessor: ULogAccessor<Output = T>,
{
    fn from_field(
        inst_field: &inst::Field,
    ) -> Result<Self, ULogError> {
        match &inst_field.value {
            inst::FieldValue::ScalarOther(inst_format) => {
                let accessor = T::from_format(&inst_format.def_format, )?;
                accessor.get_data(inst_format)
            }
            _ => {
                Err(
                    ULogError::TypeMismatch(
                        format!("expected a nested struct for field {}", inst_field.name).into(),
                    ),
                )
            }
        }
    }
}
