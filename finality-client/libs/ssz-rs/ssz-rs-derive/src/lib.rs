//! `SimpleSerialize` provides a macro to derive SSZ containers and union types from
//! native Rust structs and enums.
//! Refer to the `examples` in the `ssz_rs` crate for a better idea on how to use this derive macro.
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, Generics, Ident};

// NOTE: copied here from `ssz_rs` crate as it is unlikely to change
// and can keep it out of the crate's public interface.
const BYTES_PER_LENGTH_OFFSET: usize = 4;
const BYTES_PER_CHUNK: usize = 32;

fn derive_container_set_by_index_impl(
    name: &Ident,
    data: &Data,
    generics: &Generics,
) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            let fields = match data.fields {
                // "regular" struct with 1+ fields
                Fields::Named(ref fields) => &fields.named,
                // "tuple" struct
                // only support the case with one unnamed field, to support "newtype" pattern
                Fields::Unnamed(ref fields) => &fields.unnamed,
                _ => unimplemented!(
                    "this type of struct is currently not supported by this derive macro"
                ),
            };

            let set_by_field = fields.iter().enumerate().map(|(i, f)| {
                let field_type = &f.ty;
                match &f.ident {
                    Some(field_name) => quote_spanned! { f.span() =>
                        #i => {
                            let result = <#field_type>::deserialize(encoding)?;
                            self.#field_name = result;
                        },
                    },
                    None => quote_spanned! { f.span() =>
                        #i => {
                            let result = <#field_type>::deserialize(encoding)?;
                            self.0 = result;
                        },
                    },
                }
            });

            let impl_impl = if generics.params.is_empty() {
                quote! { #name }
            } else {
                let (_, ty_generics, _) = generics.split_for_impl();
                quote! { #generics #name #ty_generics }
            };
            quote! {
                impl #impl_impl {
                    fn __ssz_rs_set_by_index(&mut self, index: usize, encoding: &[u8]) -> Result<(), ssz_rs::DeserializeError> {
                        match index {
                            #(#set_by_field)*
                            _ => unreachable!(),
                        }
                        Ok(())
                    }
                }
            }
        }
        Data::Enum(..) => quote! {},
        Data::Union(..) => unreachable!("data was already validated to exclude union types"),
    }
}

fn derive_serialize_impl(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            let fields = match data.fields {
                // "regular" struct with 1+ fields
                Fields::Named(ref fields) => &fields.named,
                // "tuple" struct
                // only support the case with one unnamed field, to support "newtype" pattern
                Fields::Unnamed(..) => {
                    return quote! {
                        fn serialize(&self, buffer: &mut Vec<u8>) -> Result<usize, ssz_rs::SerializeError> {
                                self.0.serialize(buffer)
                        }
                    }
                }
                _ => unimplemented!(
                    "this type of struct is currently not supported by this derive macro"
                ),
            };
            let serialization_by_field = fields.iter().map(|f| {
                let field_type = &f.ty;
                match &f.ident {
                    Some(field_name) => quote_spanned! { f.span() =>
                        let mut element_buffer = Vec::with_capacity(<#field_type>::size_hint());
                        self.#field_name.serialize(&mut element_buffer)?;

                        let buffer_len = element_buffer.len();
                        if <#field_type>::is_variable_size() {
                            fixed.push(None);
                            fixed_lengths_sum += #BYTES_PER_LENGTH_OFFSET;
                            variable.push(element_buffer);
                            variable_lengths.push(buffer_len);
                        } else {
                            fixed.push(Some(element_buffer));
                            fixed_lengths_sum += buffer_len;
                            variable_lengths.push(0)
                        }
                    },
                    None => panic!("should have already returned an impl"),
                }
            });

            quote! {
                fn serialize(&self, buffer: &mut Vec<u8>) -> Result<usize, ssz_rs::SerializeError> {
                    let mut fixed = vec![];
                    let mut variable = vec![];
                    let mut variable_lengths = vec![];
                    let mut fixed_lengths_sum = 0;

                    #(#serialization_by_field)*

                    ssz_rs::__internal::serialize_composite_from_components(fixed, variable, variable_lengths, fixed_lengths_sum, buffer)
                }
            }
        }
        Data::Enum(ref data) => {
            let serialization_by_variant = data.variants.iter().enumerate().map(|(i, variant)| {
                let variant_name = &variant.ident;
                match &variant.fields {
                    Fields::Unnamed(..) => {
                        quote_spanned! { variant.span() =>
                            Self::#variant_name(value) => {
                                let selector = #i as u8;
                                let selector_bytes = selector.serialize(buffer)?;
                                let value_bytes  = value.serialize(buffer)?;
                                Ok(selector_bytes + value_bytes)
                            }
                        }
                    }
                    Fields::Unit => {
                        quote_spanned! { variant.span() =>
                            Self::None => {
                                0u8.serialize(buffer)
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            });

            quote! {
                fn serialize(&self, buffer: &mut Vec<u8>) -> Result<usize, ssz_rs::SerializeError> {
                    match self {
                        #(#serialization_by_variant)*
                    }
                }
            }
        }
        Data::Union(..) => unreachable!("data was already validated to exclude union types"),
    }
}

fn derive_deserialize_impl(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            let fields = match data.fields {
                // "regular" struct with 1+ fields
                Fields::Named(ref fields) => &fields.named,
                // "tuple" struct
                // only support the case with one unnamed field, to support "newtype" pattern
                Fields::Unnamed(ref fields) => {
                    let f = &fields.unnamed[0];
                    let field_type = &f.ty;
                    return quote! {
                        fn deserialize(encoding: &[u8]) -> Result<Self, ssz_rs::DeserializeError> {
                            let mut container = Self::default();
                            let result = <#field_type>::deserialize(&encoding)?;
                            container.0 = result;

                            Ok(container)
                        }
                    };
                }
                _ => unimplemented!(
                    "this type of struct is currently not supported by this derive macro"
                ),
            };
            let deserialization_by_field = fields.iter().enumerate().map(|(i, f)| {
                let field_type = &f.ty;
                match &f.ident {
                    Some(field_name) => quote_spanned! { f.span() =>
                        let bytes_read = if <#field_type>::is_variable_size() {
                            let end = start + #BYTES_PER_LENGTH_OFFSET;
                            let next_offset = u32::deserialize(&encoding[start..end])?;
                            offsets.push((#i, next_offset as usize));

                            #BYTES_PER_LENGTH_OFFSET
                        } else {
                            let encoded_length = <#field_type>::size_hint();
                            let end = start + encoded_length;
                            let result = <#field_type>::deserialize(&encoding[start..end])?;
                            container.#field_name = result;
                            encoded_length
                        };
                        start += bytes_read;
                    },
                    None => panic!("should have already returned an impl"),
                }
            });

            quote! {
                fn deserialize(encoding: &[u8]) -> Result<Self, ssz_rs::DeserializeError> {
                    let mut start = 0;
                    let mut offsets = vec![];
                    let mut container = Self::default();

                    #(#deserialization_by_field)*

                    let mut total_bytes_read = start;

                    // NOTE: this value is not read
                    let dummy_index = 0;
                    offsets.push((dummy_index, encoding.len()));

                    for span in offsets.windows(2) {
                        let (index, start) = span[0];
                        let (_, end) = span[1];

                        container.__ssz_rs_set_by_index(index, &encoding[start..end])?;
                        total_bytes_read += end - start;
                    }

                    if total_bytes_read > encoding.len() {
                        return Err(ssz_rs::DeserializeError::ExpectedFurtherInput {
                            provided: encoding.len(),
                            expected: total_bytes_read,
                        });
                    }

                    if total_bytes_read < encoding.len() {
                        return Err(ssz_rs::DeserializeError::AdditionalInput {
                            provided: encoding.len(),
                            expected: total_bytes_read,
                        });
                    }

                    Ok(container)
                }
            }
        }
        Data::Enum(ref data) => {
            let deserialization_by_variant =
                data.variants.iter().enumerate().map(|(i, variant)| {
                    // NOTE: this is "safe" as the number of legal variants fits into `u8`
                    let i = i as u8;
                    let variant_name = &variant.ident;
                    match &variant.fields {
                        Fields::Unnamed(inner) => {
                            let variant_type = &inner.unnamed[0];
                            quote_spanned! { variant.span() =>
                                #i => {
                                    let value = <#variant_type>::deserialize(&encoding[1..])?;
                                    Ok(Self::#variant_name(value))
                                }
                            }
                        }
                        Fields::Unit => {
                            quote_spanned! { variant.span() =>
                                0 => Ok(Self::None),
                            }
                        }
                        _ => unreachable!(),
                    }
                });

            quote! {
                fn deserialize(encoding: &[u8]) -> Result<Self, ssz_rs::DeserializeError> {
                    if encoding.is_empty() {
                        return Err(ssz_rs::DeserializeError::ExpectedFurtherInput {
                            provided: 0,
                            expected: 1,
                        });
                    }

                    match encoding[0] {
                        #(#deserialization_by_variant)*
                        b => Err(ssz_rs::DeserializeError::InvalidByte(b)),
                    }
                }
            }
        }
        Data::Union(..) => unreachable!("data was already validated to exclude union types"),
    }
}

fn derive_variable_size_impl(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            let fields = match data.fields {
                Fields::Named(ref fields) => &fields.named,
                Fields::Unnamed(ref fields) => &fields.unnamed,
                _ => unimplemented!(
                    "this type of struct is currently not supported by this derive macro"
                ),
            };
            let impl_by_field = fields.iter().map(|f| {
                let field_type = &f.ty;
                quote_spanned! { f.span() =>
                    <#field_type>::is_variable_size()
                }
            });

            quote! {
                #(#impl_by_field)|| *
            }
        }
        Data::Enum(..) => {
            quote! { true }
        }
        Data::Union(..) => unreachable!("data was already validated to exclude union types"),
    }
}

fn derive_size_hint_impl(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            let fields = match data.fields {
                Fields::Named(ref fields) => &fields.named,
                Fields::Unnamed(ref fields) => &fields.unnamed,
                _ => unimplemented!(
                    "this type of struct is currently not supported by this derive macro"
                ),
            };
            let impl_by_field = fields.iter().map(|f| {
                let field_type = &f.ty;
                quote_spanned! { f.span() =>
                    <#field_type>::size_hint()
                }
            });

            quote! {
                if Self::is_variable_size() {
                    0
                } else {
                    #(#impl_by_field)+ *
                }
            }
        }
        Data::Enum(..) => {
            quote! { 0 }
        }
        Data::Union(..) => unreachable!("data was already validated to exclude union types"),
    }
}

fn derive_merkleization_impl(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            let fields = match data.fields {
                Fields::Named(ref fields) => &fields.named,
                Fields::Unnamed(ref fields) => &fields.unnamed,
                _ => unimplemented!(
                    "this type of struct is currently not supported by this derive macro"
                ),
            };
            let field_count = fields.iter().len();
            let impl_by_field = fields.iter().enumerate().map(|(i, f)| match &f.ident {
                Some(field_name) => quote_spanned! { f.span() =>
                    let chunk = self.#field_name.hash_tree_root()?;
                    let range = #i*#BYTES_PER_CHUNK..(#i+1)*#BYTES_PER_CHUNK;
                    chunks[range].copy_from_slice(chunk.as_ref());
                },
                None => quote_spanned! { f.span() =>
                    let chunk = self.0.hash_tree_root()?;
                    let range = #i*#BYTES_PER_CHUNK..(#i+1)*#BYTES_PER_CHUNK;
                    chunks[range].copy_from_slice(chunk.as_ref());
                },
            });
            quote! {
                fn hash_tree_root(&mut self) -> Result<ssz_rs::Node, ssz_rs::MerkleizationError> {
                    let mut chunks = vec![0u8; #field_count * #BYTES_PER_CHUNK];
                    #(#impl_by_field)*
                    ssz_rs::__internal::merkleize(&chunks, None)
                }
            }
        }
        Data::Enum(ref data) => {
            let hash_tree_root_by_variant = data.variants.iter().enumerate().map(|(i, variant)| {
                let variant_name = &variant.ident;
                match &variant.fields {
                    Fields::Unnamed(..) => {
                        quote_spanned! { variant.span() =>
                            Self::#variant_name(value) => {
                                let selector = #i;
                                let data_root  = value.hash_tree_root()?;
                                Ok(ssz_rs::__internal::mix_in_selector(&data_root, selector))
                            }
                        }
                    }
                    Fields::Unit => {
                        quote_spanned! { variant.span() =>
                            Self::None => Ok(ssz_rs::__internal::mix_in_selector(
                                &ssz_rs::Node::default(),
                                0,
                            )),
                        }
                    }
                    _ => unreachable!(),
                }
            });
            quote! {
                fn hash_tree_root(&mut self) -> Result<ssz_rs::Node, ssz_rs::MerkleizationError> {
                    match self {
                            #(#hash_tree_root_by_variant)*
                    }
                }
            }
        }
        Data::Union(..) => unreachable!("data was already validated to exclude union types"),
    }
}

fn derive_treeify_impl(data: &Data) -> TokenStream {
    match data {
        Data::Struct(ref data) => {
            let fields = match data.fields {
                Fields::Named(ref fields) => &fields.named,
                Fields::Unnamed(ref fields) => &fields.unnamed,
                _ => unimplemented!(
                    "this type of struct is currently not supported by this derive macro"
                ),
            };
            let field_count = fields.iter().len();
            let impl_by_field = fields.iter().enumerate().map(|(i, f)| match &f.ident {
                Some(field_name) => quote_spanned! { f.span() =>
                    let chunk = self.#field_name.hash_tree_root()?;
                    let mut sub_tree = self.#field_name.to_merkle_tree()?;

                    let range = #i*#BYTES_PER_CHUNK..(#i+1)*#BYTES_PER_CHUNK;
                    chunks[range].copy_from_slice(chunk.as_ref());
                    tree.append(&mut sub_tree);
                },
                None => quote_spanned! { f.span() =>
                    let chunk = self.0.hash_tree_root()?;
                    let mut sub_tree = self.0.to_merkle_tree()?;

                    let range = #i*#BYTES_PER_CHUNK..(#i+1)*#BYTES_PER_CHUNK;
                    chunks[range].copy_from_slice(chunk.as_ref());
                    tree.append(&mut sub_tree);
                },
            });
            quote! {
                fn to_merkle_tree(&mut self) -> Result<Vec<([u8;32], [u8;64])>, ssz_rs::MerkleizationError> {
                    let mut chunks = vec![0u8; #field_count * #BYTES_PER_CHUNK];
                    let mut tree = vec![];
                    #(#impl_by_field)*
                    let new_nodes = &mut ssz_rs::__internal::treeify(&chunks, None)?;
                    tree.append(new_nodes);
                    Ok(tree)
                }
            }
        }
        Data::Enum(ref data) => {
            let hash_tree_root_by_variant = data.variants.iter().enumerate().map(|(i, variant)| {
                let variant_name = &variant.ident;
                match &variant.fields {
                    Fields::Unnamed(..) => {
                        quote_spanned! { variant.span() =>
                            Self::#variant_name(value) => {
                                let mut tree = value.to_merkle_tree()?;

                                let selector = #i;
                                let data_root = value.hash_tree_root()?;
                                tree.push(ssz_rs::__internal::mix_in_selector_tree(&data_root, selector));
                                

                                Ok(tree)
                            }
                        }
                    }
                    Fields::Unit => {
                        quote_spanned! { variant.span() =>
                            Self::None => {
                                Ok(vec![ssz_rs::__internal::mix_in_selector_tree(
                                    &ssz_rs::Node::default(),
                                    0,
                                )])
                            }
                        }
                    }
                    _ => unreachable!(),
                }
            });
            quote! {
                fn to_merkle_tree(&mut self) -> Result<Vec<([u8;32], [u8;64])>, ssz_rs::MerkleizationError> {
                    match self {
                            #(#hash_tree_root_by_variant)*
                    }
                }
            }
        }
        Data::Union(..) => unreachable!("data was already validated to exclude union types"),
    }
}

fn is_valid_none_identifier(ident: &Ident) -> bool {
    *ident == format_ident!("None")
}

// Refers to the validation state of proc macro's input
enum ValidationState<'a> {
    Unvalidated(&'a Data),
    Validated(&'a Data),
}

// Validates the incoming data follows the rules
// for mapping the Rust term to something that can
// implement the `SimpleSerialize` trait.
fn validate_derive_data(data: ValidationState) -> ValidationState {
    let data = match data {
        ValidationState::Unvalidated(data) => data,
        data @ ValidationState::Validated(..) => return data,
    };

    match data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                if fields.named.is_empty() {
                    panic!("ssz_rs containers with no fields are illegal")
                }
            }
            Fields::Unnamed(ref fields) if fields.unnamed.len() == 1 => {}
            _ => panic!("Structs with unit or multiple unnnamed fields are not supported"),
        },
        Data::Enum(ref data) => {
            if data.variants.is_empty() {
                panic!("SSZ unions must have at least 1 variant; this enum has none");
            }

            if data.variants.len() > 127 {
                panic!("SSZ unions cannot have more than 127 variants; this enum has more");
            }

            let mut none_forbidden = false;
            let mut already_has_none = false;
            for (i, variant) in data.variants.iter().enumerate() {
                match &variant.fields {
                    Fields::Unnamed(inner) => {
                        if i == 0 {
                            none_forbidden = true;
                        }
                        if inner.unnamed.len() != 1 {
                            panic!("Enums can only have 1 type per variant");
                        }
                    }
                    Fields::Unit => {
                        if none_forbidden {
                            panic!("only the first variant can be `None`");
                        }
                        if already_has_none {
                            panic!("cannot duplicate a unit variant (as only `None` is allowed)");
                        }
                        if !is_valid_none_identifier(&variant.ident) {
                            panic!("Variant identifier is invalid: must be `None`");
                        }
                        assert!(i == 0);
                        if data.variants.len() < 2 {
                            panic!(
                                "SSZ unions must have more than 1 selector if the first is `None`"
                            );
                        }
                        already_has_none = true;
                    }
                    Fields::Named(..) => {
                        panic!("Enums with named fields in variants are not supported");
                    }
                };
            }
        }
        Data::Union(..) => panic!("Rust unions cannot produce valid SSZ types"),
    }

    ValidationState::Validated(data)
}

#[proc_macro_derive(SimpleSerialize)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let data = ValidationState::Unvalidated(&input.data);

    let data = &validate_derive_data(data);

    let data = match *data {
        ValidationState::Validated(data) => data,
        ValidationState::Unvalidated(..) => panic!("do not process unvalidated input"),
    };

    let name = &input.ident;
    let generics = &input.generics;
    let set_by_index_impl = derive_container_set_by_index_impl(name, data, generics);
    let serialize_impl = derive_serialize_impl(data);
    let deserialize_impl = derive_deserialize_impl(data);
    let is_variable_size_impl = derive_variable_size_impl(data);
    let size_hint_impl = derive_size_hint_impl(data);
    let merkleization_impl = derive_merkleization_impl(data);
    let treeify_impl = derive_treeify_impl(data);

    let impl_impl = if generics.params.is_empty() {
        quote! { impl }
    } else {
        quote! { impl #generics }
    };

    let name_impl = if generics.params.is_empty() {
        quote! { #name }
    } else {
        let (_, ty_generics, _) = generics.split_for_impl();
        quote! { #name #ty_generics }
    };

    let expansion = quote! {
        #set_by_index_impl

        #impl_impl ssz_rs::Serialize for #name_impl {
            #serialize_impl
        }

        #impl_impl ssz_rs::Deserialize for #name_impl {
            #deserialize_impl
        }

        #impl_impl ssz_rs::Sized for #name_impl {
            fn is_variable_size() -> bool {
                #is_variable_size_impl
            }

            fn size_hint() -> usize {
                #size_hint_impl
            }
        }

        #impl_impl ssz_rs::Merkleized for #name_impl {
            #merkleization_impl
            #treeify_impl
        }

        #impl_impl ssz_rs::SimpleSerialize for #name_impl {}
    };

    proc_macro::TokenStream::from(expansion)
}
