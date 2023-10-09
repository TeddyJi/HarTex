/*
 * SPDX-License-Identifier: AGPL-3.0-only
 *
 * This file is part of HarTex.
 *
 * HarTex
 * Copyright (c) 2021-2023 HarTex Project Developers
 *
 * HarTex is free software; you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * HarTex is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License along
 * with HarTex. If not, see <https://www.gnu.org/licenses/>.
 */

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::Expr;
use syn::ExprArray;
use syn::ExprLit;
use syn::ItemStruct;
use syn::Lit;
use syn::LitStr;
use syn::Token;
use syn::Type;

use crate::metadata;

// FIXME: this needs to be reimagined
#[allow(dead_code)]
#[allow(clippy::module_name_repetitions)]
pub struct EntityMacroInput {
    from_ident: Ident,
    equal1: Token![=],
    from_lit_str: LitStr,
    comma1: Token![,],
    exclude_or_include_ident: Ident,
    equal2: Token![=],
    exclude_or_include_array: ExprArray,
    comma2: Option<Token![,]>,
}

impl Parse for EntityMacroInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            from_ident: input.parse()?,
            equal1: input.parse()?,
            from_lit_str: input.parse()?,
            comma1: input.parse()?,
            exclude_or_include_ident: input.parse()?,
            equal2: input.parse()?,
            exclude_or_include_array: input.parse()?,
            comma2: input.parse().ok(),
        })
    }
}

#[allow(clippy::module_name_repetitions)]
#[allow(clippy::too_many_lines)]
pub fn implement_entity(input: &EntityMacroInput, item_struct: &ItemStruct) -> Option<TokenStream> {
    if input.from_ident != "from" {
        input
            .from_ident
            .span()
            .unwrap()
            .error("expected `from`")
            .emit();

        return None;
    }

    let type_key = input.from_lit_str.value();
    if !metadata::STRUCT_MAP.contains_key(type_key.as_str()) {
        input
            .from_lit_str
            .span()
            .unwrap()
            .error(format!("type `{type_key}` cannot be found"))
            .note(format!(
                "the type metadata generated was for twilight-model version {}",
                metadata::CRATE_VERSION
            ))
            .help("consider regenerating the metadata for a newer version if the type is recently added")
            .emit();

        return None;
    }

    let type_metadata = metadata::STRUCT_MAP
        .get(type_key.as_str())
        .copied()
        .cloned()
        .unwrap();
    let mut any_not_found = false;
    let fields = input
        .exclude_or_include_array
        .elems
        .iter()
        .filter_map(|expr| match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Str(lit_str),
                ..
            }) => {
                if type_metadata
                    .fields
                    .iter()
                    .any(|field| field.name == lit_str.value())
                {
                    Some(lit_str.value())
                } else {
                    lit_str
                        .span()
                        .unwrap()
                        .error(format!("field `{}` cannot be found in type `{type_key}`", lit_str.value()))
                        .note(format!(
                            "the type metadata generated was for twilight-model version {}",
                            metadata::CRATE_VERSION
                        ))
                        .help("consider regenerating the metadata for a newer version if the field is recently added")
                        .emit();
                    any_not_found = true;

                    None
                }
            }
            expr => {
                expr.span()
                    .unwrap()
                    .warning("non-string expressions are ignored")
                    .emit();

                None
            }
        })
        .collect::<Vec<_>>();

    if any_not_found {
        return None;
    }

    let item_struct_vis = item_struct.vis.clone();
    let item_struct_name = item_struct.ident.clone();

    match input.exclude_or_include_ident.to_string().as_str() {
        "exclude" => {
            let fields = type_metadata
                .fields
                .iter()
                .filter_map(|field| {
                    if !fields.contains(&field.name) {
                        None
                    } else {
                        let field_name = Ident::new(field.name.as_str(), Span::call_site());
                        let field_type = syn::parse_str::<Type>(field.ty.as_str()).unwrap();

                        Some(quote! {#field_name: #field_type})
                    }
                })
                .collect::<Vec<_>>();

            Some(quote! {
                #item_struct_vis struct #item_struct_name {
                    #(#fields),*
                }
            })
        }
        "include" => {
            let fields = type_metadata
                .fields
                .iter()
                .filter_map(|field| {
                    if fields.contains(&field.name) {
                        let field_name = Ident::new(field.name.as_str(), Span::call_site());
                        let field_type = syn::parse_str::<Type>(field.ty.as_str()).unwrap();

                        Some(quote! {#field_name: #field_type})
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            Some(quote! {
                #item_struct_vis struct #item_struct_name {
                    #(#fields),*
                }
            })
        }
        _ => {
            input
                .exclude_or_include_ident
                .span()
                .unwrap()
                .error("expected `exclude` or `include`")
                .emit();

            None
        }
    }
}
