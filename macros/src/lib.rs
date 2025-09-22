//! # My Crate: ULog Macros
//!
//! Provides `#[derive(ULogData)]` and `#[derive(ULogMessages)]` for mapping Rust structs
//! to ULOG subscriptions.
//!
//! # Example
//! ```rust
//! #[cfg(feature = "macros")]  {
//! extern crate yule_log_macros;
//! use yule_log_macros::{ULogData, ULogMessages};
//!
//! #[derive(ULogData)]
//! struct VehicleLocalPosition { timestamp: u64, x: f32, y: f32, z: f32 }
//!
//! #[derive(ULogMessages)]
//! enum LoggedMessages {
//!     VehicleLocalPosition(VehicleLocalPosition),
//!     Other(yule_log::model::msg::UlogMessage),
//! }
//!
//! let reader = std::io::BufReader::new(std::fs::File::open("test_data/sample.ulg")?);
//! let stream = LoggedMessages::stream(reader)?;
//!
//! for msg_res in stream {
//!     let msg = msg_res?;
//!     match msg {
//!         LoggedMessages::VehicleLocalPosition(v) => println!("x={}", v.x),
//!         LoggedMessages::Other(_) => {},
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! }
//! ```

mod utils;

use syn::spanned::Spanned;
use proc_macro::TokenStream;
use syn::{DeriveInput, Ident, Type};
use darling::FromDeriveInput;
use darling::FromVariant;
use darling::FromField;
use heck::ToSnakeCase;
use quote::quote;
use crate::utils::{extract_option_type, is_option_type, make_index_type};
//
// --------------------------- Struct Derive ---------------------------
//

#[derive(FromDeriveInput)]
#[darling(attributes(yule_log))]
struct LoggedStructAttr {
    /// Optional subscription name override. Defaults to lower snake case of struct name.
    subscription_name: Option<String>,
    #[darling(default)]
    /// Optional multi_id for subscriptions with multiple instances.
    multi_id: Option<u8>,
}

#[derive(FromField, Default)]
#[darling(attributes(yule_log))]
struct LoggedFieldAttr {
    #[darling(default)]
    /// Optional field name override for ULog mapping. Defaults to struct field name.
    field_name: Option<String>,
}

/// Derive `ULogData` for a struct representing a ULOG LoggedDataMessage.
///
/// # Attributes
///
/// * `#[yule_log(subscription_name = "...")]` – optional override for the subscription name.
///   By default, the subscription name is derived from the struct name by converting it to lower snake case.
///
/// * `#[yule_log(multi_id = N)]` – optional multi-instance ID. Defaults to `0` if not set.
///
/// Each field can also use an optional attribute:
///
/// * `#[yule_log(field_name = "...")]` – override the field name used in the ULOG message.
///   Defaults to the struct field name.
///
/// # Example
///
/// ```ignore
/// #[derive(ULogData)]
/// #[yule_log(subscription_name = "vehicleLocalPosition")]
/// pub struct VehicleLocalPosition {
///     timestamp: u64,
///     x: f32,
///     y: f32,
///     z: f32,
/// }
///
#[proc_macro_derive(ULogData, attributes(yule_log))]
pub fn derive_logged_struct(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    // Can only be applied to structs.
    let fields = if let syn::Data::Struct(data) = &input.data {
        data.fields.iter().collect::<Vec<_>>()
    } else {
        return syn::Error::new_spanned(
            input.ident,
            "ULogData derive on structs only",
        )
            .to_compile_error()
            .into();
    };

    // Issue a compile error if the user struct has generics or where clauses.  These are not supported yet.
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return syn::Error::new_spanned(
            input.ident,
            "ULogData derive cannot be used with generic structs or where clauses."
        )
            .to_compile_error()
            .into();
    }

    // Require that each struct contain only named fields.
    for f in &fields {
        if f.ident.is_none() {
            return syn::Error::new_spanned(
                f,
                "ULogData requires named struct fields.",
            )
                .to_compile_error()
                .into();
        }
    }

    let struct_name = &input.ident;

    // Parse optional subscription name + multi_id
    let attr = match LoggedStructAttr::from_derive_input(&input) {
        Ok(a) => a,
        Err(e) => return e.write_errors().into(),
    };
    let subscription = attr
        .subscription_name
        .unwrap_or_else(|| struct_name.to_string().to_snake_case());
    let multi_id = attr.multi_id.unwrap_or(0);

    let accessor_name = Ident::new(&format!("{}Accessor", struct_name), struct_name.span());

    fn named_ident(f: &syn::Field) -> &syn::Ident {
        #[allow(clippy::unwrap_used)] // Safe by invariant: all fields are named. See guard code above.
        f.ident.as_ref().unwrap()
    }

    fn idx_ident(f: &syn::Field) -> Ident {
        let name = named_ident(f);
        Ident::new(&format!("{}_index", name), name.span())
    }

    fn idx_type(f: &syn::Field) -> Type {
        // If the user type is an Option<T>, will generate Option<usize>
        // otherwise usize.
        make_index_type(f)
    }
    
    // Generate index_fields to hold the index of the field in the LoggedData message, for efficient lookup.
    let index_fields = fields.iter().map(|f| {
        let idx_ident = idx_ident(f);
        let idx_type = idx_type(f);
        quote! { #idx_ident: #idx_type }
    });

    let accessor_struct = {
        let idx_field_exprs = fields.iter().map(|f| {
            let idx_ident = idx_ident(f);
            let ulog_name = {
                // This unwrap is safe because LoggedFieldAttr has the `Default` attribute applied.
                let attr = LoggedFieldAttr::from_field(f).unwrap();
                attr.field_name.unwrap_or_else(|| named_ident(f).to_string())
            };

            if is_option_type(&make_index_type(f)) {
                quote! {
                    // If not found store index value as None, otherwise Some(usize).
                    #idx_ident: map.get(#ulog_name).copied()
                }
            } else {
                quote! {
                #idx_ident: match map.get(#ulog_name) {
                    Some(&idx) => idx,
                    None => {
                        // If not found add to missing list to report an error later.
                        // Index is set to 0, but will never be used.
                        missing.push(#ulog_name);
                        0
                    }
                }
            }
            }
        });

        quote! {
        {
            let map: std::collections::HashMap<String, usize> =
                format.fields.iter().enumerate()
                    .map(|(i, f)| (f.name.clone(), i))
                    .collect();

            let mut missing = Vec::new();

            let result = Self {
                #( #idx_field_exprs ),*
            };

            if !missing.is_empty() {
                return Err(yule_log::errors::ULogError::InvalidFieldName(
                    format!(
                        "The following fields were not found in subscription `{}`: {}",
                        #subscription,
                        missing.join(", ")
                    )
                ));
            } else {
                return Ok(result);
            }
        }
    }
    };

    let from_field_path: syn::Path = syn::parse_str("FromField").unwrap();

    // Generate get_data fields using the FromField trait.
    //
    // Assumptions:
    //
    // - For any field `f` of type `Option<T>`, the corresponding index field `self.#idx_ident` is `Option<usize>`,
    //   indicating whether the field is present in the ULog format (None means the field is not present).
    // - For any non-optional field `f`, the corresponding index field is `usize` and guaranteed to be present.
    //
    // This invariant is established earlier when generating index fields and validated by construction of the Accessor struct.
    // Therefore, it is safe to unwrap and index into `format.fields` accordingly.
    //
    // The generated code uses this to:
    // - Return `None` directly when the index is `None` (field missing),
    // - Or call `FromField` on the inner type if present,
    // - Or call `FromField` on the full type for non-optional fields.
    let get_data_fields = 
        fields.iter().map(|f| {
            let name = named_ident(f);
            let idx_ident = idx_ident(f);
            let ty = &f.ty;

            if is_option_type(ty) {
                // This unwrap() is safe because we just confirmed it's an Option.
                let inner_ty = extract_option_type(ty).expect("Expected Option inner type.");

                quote! {
                    #name: match self.#idx_ident {
                        None => None,
                        Some(idx) => Some(<#inner_ty as #from_field_path>::from_field(&format.fields[idx])?)
                    }
                }
            } else {
                quote! {
                    #name: <#ty as #from_field_path>::from_field(&format.fields[self.#idx_ident])?
                }
            }
    });

    let expanded = quote! {
        #[doc = "Represents the mapping of a ULOG LoggedDataMessage."]
        #[doc = concat!("Subscription name: ", #subscription)]
        #[doc = "The subscription name is derived by lower snake case of the struct name, unless overridden via #[yule_log(subscription_name=\"...\")]."]
        #[doc = "Optional #[yule_log(multi_id = N)] selects a multi-instance subscription."]
        #[automatically_derived]
        impl #struct_name {
            const __YULE_LOG_SUBSCRIPTION: &'static str = #subscription;
            const __YULE_LOG_MULTI_ID: u8 = #multi_id;
        }

        #[doc = "Accessor type for efficiently retrieving fields from this message type."]
        #[automatically_derived]
        pub struct #accessor_name {
            #( #index_fields ),*
        }

        #[automatically_derived]
        impl #accessor_name {
            pub(crate) fn from_format(format: &yule_log::model::def::Format) -> Result<Self, yule_log::errors::ULogError> {
                #accessor_struct
            }
        }


        #[automatically_derived]
        impl yule_log::macro_utils::ULogAccessorFactory for #struct_name {
            type Accessor = #accessor_name;

            fn from_format(format: &yule_log::model::def::Format)
                -> Result<Self::Accessor, yule_log::errors::ULogError>
            {
                #accessor_name::from_format(format)
            }
        }

        #[automatically_derived]
        impl yule_log::macro_utils::ULogAccessor for #accessor_name {
            type Output = #struct_name;

            fn get_data(&self, format: &yule_log::model::inst::Format)
                -> Result<Self::Output, yule_log::errors::ULogError>
            {
                use ::yule_log::macro_utils::FromField;

                Ok(#struct_name {
                    #( #get_data_fields ),*
                })
            }
        }

    };

    expanded.into()
}

//
// --------------------------- Enum Derive ---------------------------
//

#[derive(FromVariant, Default)]
#[darling(attributes(yule_log))]
// Tracks which enum variant is marked #[yule_log(forward_other)] at macro expansion
struct LoggedVariantAttr {
    #[darling(default)]
    /// Marks a variant that will receive unrecognized messages.
    forward_other: bool,
}

/// Derive `ULogMessages` for an enum wrapping ULogData structs.
///
/// This enum acts as the dispatcher for all ULOG message types.
/// It generates an internal iterator over the messages from a ULOG reader.
///
/// # Attributes
///
/// * `#[yule_log(forward_other)]` – marks a variant that receives unmapped messages.
///   Only one variant may have this attribute.
///
/// # Example
///
/// ```ignore
/// #[derive(ULogMessages)]
/// pub enum LoggedMessages {
///     VehicleLocalPosition(VehicleLocalPosition),
///     ActuatorOutputs(ActuatorOutputs),
///     #[yule_log(forward_other)]
///     Other(yule_log::model::msg::UlogMessage),
/// }
/// ```
#[proc_macro_derive(ULogMessages, attributes(yule_log))]
pub fn derive_logged_enum(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);

    let enum_name = &input.ident;

    let variants = if let syn::Data::Enum(data_enum) = &input.data {
        &data_enum.variants
    } else {
        return syn::Error::new_spanned(
            input.ident,
            "ULogMessages derive only works on enums.",
        )
            .to_compile_error()
            .into();
    };

    // Issue a compile error if the user enum has generics or where clauses.  These are not supported yet.
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return syn::Error::new_spanned(
            input.ident,
            "ULogMessages derive cannot be used with generic enums or where clauses."
        )
            .to_compile_error()
            .into();
    }

    // Find the variant marked with #[yule_log(forward_other)], if present
    let mut forward_other_variant_ident = None;
    let mut variant_info = Vec::new();

    for v in variants {
        let attr = LoggedVariantAttr::from_variant(v).unwrap_or_default();
        if attr.forward_other {
            if forward_other_variant_ident.is_some() {
                return syn::Error::new_spanned(
                    v,
                    "Only one variant may have #[yule_log(forward_other)]."
                )
                    .to_compile_error()
                    .into();
            }
            forward_other_variant_ident = Some(v.ident.clone());
        }

        if let syn::Fields::Unnamed(fields) = &v.fields {
            if fields.unnamed.len() != 1 {
                return syn::Error::new_spanned(
                    v,
                    "Enum variants must be tuple variants containing exactly one struct type.",
                )
                    .to_compile_error()
                    .into();
            }
            variant_info.push((v.ident.clone(), &fields.unnamed[0].ty));
        }
    }

    // Filter out forward_other variant if present
    let filtered_variants: Vec<_> = variant_info
        .iter()
        .filter(|(var, _)| Some(var.clone()) != forward_other_variant_ident)
        .collect();

    let accessor_enum_name = Ident::new(
        &format!("__yule_log_derive_{}Accessor", enum_name),
        enum_name.span(),
    );

    let hidden_struct_name = Ident::new(
        &format!("__yule_log_derive_{}", enum_name),
        enum_name.span(),
    );

    /// Generate an accessor name for an enum variant wrapping a single struct.
    /// Fails at compile time if the type is not a simple struct path.
    fn generate_accessor_name(ty: &syn::Type, span: proc_macro2::Span) -> Result<Ident, syn::Error> {
        if let syn::Type::Path(type_path) = ty {
            #[allow(clippy::unwrap_used)] // Safe: syn::Type::Path always has at least one segment for a valid Rust struct type.
            let ident = &type_path.path.segments.last().unwrap().ident;
            let name = format!("{}Accessor", ident);
            Ok(Ident::new(&name, span))
        } else {
            Err(syn::Error::new(span, "Enum variants must be tuple variants containing exactly one struct type."))
        }
    }

    // Generate variant accessors
    let accessor_enum_variants: Result<Vec<_>, syn::Error> = filtered_variants
        .iter()
        .map(|(var, ty)| {
            generate_accessor_name(ty, ty.span())
                .map(|accessor_ident| quote! { #var(#accessor_ident) })
        })
        .collect();

    let accessor_enum_variants = match accessor_enum_variants {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    // Generate match arms for AddSubscription with fallible error propagation
    let add_subscription_arms: Vec<_> = filtered_variants
        .iter()
        .map(|(var, ty)| {
            quote! {
                (<#ty>::__YULE_LOG_SUBSCRIPTION, <#ty>::__YULE_LOG_MULTI_ID) => {
                    let format = match self.parser.get_format(&sub.message_name) {
                        Ok(f) => f,
                        Err(e) => return Some(Err(yule_log::errors::ULogError::from(e))),
                    };

                    let acc = match <#ty as yule_log::macro_utils::ULogAccessorFactory>::from_format(&format) {
                        Ok(a) => a,
                        Err(e) => return Some(Err(e)),
                    };

                    self.subs.insert(
                        sub.msg_id,
                        #accessor_enum_name::#var(acc),
                    );
                }
            }
        })
        .collect();

    // Generate match arms for LoggedData (only yields Result)
    let logged_data_arms: Vec<_> = filtered_variants
        .iter()
        .map(|(var, _ty)| {
            quote! {
                #accessor_enum_name::#var(a) => a.get_data(&data.data).map(|v| #enum_name::#var(v))
            }
        })
        .collect();

    // Build subscription allow-list tokens (names only, as parser doesn't know about multi_id)
    let subscription_idents: Vec<_> = filtered_variants
        .iter()
        .map(|(_var, ty)| quote! { #ty::__YULE_LOG_SUBSCRIPTION })
        .collect();

    // Generate match arm for forwarding other messages
    let forward_other_arm = if let Some(forward_ident) = &forward_other_variant_ident {
        quote! {
            _ => {
                return Some(Ok(#enum_name::#forward_ident(msg)));
            }
        }
    } else {
        quote! {
            _ => {} // safe no-op when no forward_other variant
        }
    };



    let expanded = quote! {
        #[doc = "Internal enum holding accessors for each variant."]
        #[allow(non_camel_case_types)]
        #[automatically_derived]
        enum #accessor_enum_name {
            #( #accessor_enum_variants ),*
        }

        #[doc = "Internal iterator struct driving the ULog parser and dispatching messages."]
        #[allow(non_camel_case_types)]
        #[automatically_derived]
        struct #hidden_struct_name<R: std::io::Read> {
            parser: yule_log::parser::ULogParser<R>,
            subs: std::collections::HashMap<u16, #accessor_enum_name>,
        }

        #[automatically_derived]
        impl<R: std::io::Read> #hidden_struct_name<R> {
            fn new(reader: R) -> Result<Self, yule_log::errors::ULogError> {
                let mut parser = yule_log::builder::ULogParserBuilder::new(reader)
                    .include_timestamp(true)
                    .include_padding(true)
                    .build()
                    .map_err(|e| yule_log::errors::ULogError::InternalError(e.to_string()))?;

                // Set allow-list from all subscription names in user structs
                let allowed_subs: std::collections::HashSet<String> =
                    [ #( #subscription_idents.to_string() ),* ].into_iter().collect();

                parser.set_subscription_allow_list(allowed_subs);

                Ok(Self { parser, subs: std::collections::HashMap::new() })
            }
        }

        #[automatically_derived]
        impl<R: std::io::Read> Iterator for #hidden_struct_name<R> {
            type Item = Result<#enum_name, yule_log::errors::ULogError>;

            fn next(&mut self) -> Option<Self::Item> {
                use ::yule_log::macro_utils::ULogAccessorFactory;
                use ::yule_log::model::msg::UlogMessage;
                use ::yule_log::macro_utils::ULogAccessor;

                while let Some(msg_res) = self.parser.next() {
                    let msg = match msg_res {
                        Ok(m) => m,
                        Err(e) => return Some(Err(yule_log::errors::ULogError::from(e))),
                    };

                    match msg {
                        UlogMessage::AddSubscription(sub) => {
                            match (sub.message_name.as_str(), sub.multi_id) {
                                #( #add_subscription_arms )*
                                _ => {}
                            }
                            // Continue looping; don't yield yet
                        }
                        UlogMessage::LoggedData(data) => {
                            if let Some(acc) = self.subs.get(&data.msg_id) {
                                return Some(match acc {
                                    #( #logged_data_arms ),*
                                });
                            }
                        }
                        #forward_other_arm
                    }
                }

                None
            }
        }
        
        #[automatically_derived]
        impl #enum_name {
            #[doc = "Returns an iterator over the selected ULOG messages from the reader."]
            pub fn stream<R: std::io::Read>(
                reader: R,
            ) -> Result<impl Iterator<Item = Result<Self, yule_log::errors::ULogError>>, yule_log::errors::ULogError>
            {
                #hidden_struct_name::new(reader)
            }
        }
    };

    expanded.into()
}
