use crate::errors::ULogError;
use crate::model::inst::{Field, FieldValue};
use crate::model::inst;


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
            fn from_field(field: &Field) -> Result<Self, ULogError> {
                match &field.value {
                    FieldValue::$variant(v) => Ok(*v),
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
            fn from_field(field: &Field) -> Result<Self, ULogError> {
                match &field.value {
                    FieldValue::$variant(v) => Ok(v.clone()),
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



/// Trait for deriving accessors for logged data.
pub trait ULogAccess {
    type Accessor;

    /// Create an accessor from the subscription definition format.
    fn from_format(format: &crate::model::def::Format) -> Result<Self::Accessor, crate::errors::ULogError>;
}


