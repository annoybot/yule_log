use crate::errors::ULogError;
use crate::model::inst::{Field, FieldValue, Format};
use crate::model::inst;


/// Trait for deriving accessors for logged data.
pub trait ULogAccess {
    type Accessor;

    /// Create an accessor from the subscription definition format.
    fn from_format(format: &crate::model::def::Format) -> Result<Self::Accessor, crate::errors::ULogError>;
}
