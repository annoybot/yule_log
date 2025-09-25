use proc_macro2::Span;
use syn::{
    Field, GenericArgument, Ident, Path, PathArguments, PathSegment, Token, Type, TypePath,
};

pub fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            if seg.ident == "Option" {
                return matches!(seg.arguments, PathArguments::AngleBracketed(_));
            }
        }
    }
    false
}

pub fn make_usize_type() -> Type {
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments: {
                let mut segments = syn::punctuated::Punctuated::new();
                segments.push(PathSegment {
                    ident: Ident::new("usize", Span::call_site()),
                    arguments: PathArguments::None,
                });
                segments
            },
        },
    })
}

pub fn make_option_usize_type() -> Type {
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments: {
                let mut segments = syn::punctuated::Punctuated::new();
                segments.push(PathSegment {
                    ident: Ident::new("Option", Span::call_site()),
                    arguments: PathArguments::AngleBracketed(
                        syn::AngleBracketedGenericArguments {
                            colon2_token: None,
                            lt_token: <Token![<]>::default(),
                            gt_token: <Token![>]>::default(),
                            args: {
                                let mut args = syn::punctuated::Punctuated::new();
                                args.push(GenericArgument::Type(make_usize_type()));
                                args
                            },
                        },
                    ),
                });
                segments
            },
        },
    })
}

pub fn make_index_type(field: &Field) -> Type {
    if is_option_type(&field.ty) {
        make_option_usize_type()
    } else {
        make_usize_type()
    }
}

pub fn extract_option_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(PathSegment { ident, arguments }) = path.segments.last() {
            if *ident == "Option" {
                if let PathArguments::AngleBracketed(ref args) = arguments {
                    if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
}

