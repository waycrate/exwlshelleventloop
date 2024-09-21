extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use syn::{
    parse_macro_input, Data, DataEnum, DeriveInput, Expr, ExprLit, Lit, Meta, MetaNameValue,
};

use quote::quote;

fn find_attribute_values(ast: &syn::DeriveInput, attr_name: &str) -> Vec<String> {
    ast.attrs
        .iter()
        .filter(|value| value.path().is_ident(attr_name))
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(MetaNameValue {
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(val), ..
                    }),
                ..
            }) => Some(val.value()),
            _ => None,
        })
        .collect()
}

fn find_attribute_values_boolean(ast: &syn::DeriveInput, attr_name: &str) -> Vec<bool> {
    ast.attrs
        .iter()
        .filter(|value| value.path().is_ident(attr_name))
        .filter_map(|attr| match &attr.meta {
            Meta::NameValue(MetaNameValue {
                value:
                    Expr::Lit(ExprLit {
                        lit: Lit::Bool(val),
                        ..
                    }),
                ..
            }) => Some(val.value()),
            _ => None,
        })
        .collect()
}

fn impl_layer_action_message(ast: &syn::DeriveInput) -> syn::Result<TokenStream2> {
    let Data::Enum(DataEnum { variants, .. }) = &ast.data else {
        return Err(syn::Error::new_spanned(
            ast,
            "RustEmbed can only be derived for enum",
        ));
    };

    let enum_identifer = &ast.ident;
    let is_multi = find_attribute_values_boolean(ast, "multi")
        .last()
        .unwrap_or(&false)
        .to_owned();

    let new_enum_identifier = Ident::new(
        &format!("{}LayerMessage", enum_identifer.to_string()),
        Span::call_site(),
    );

    let new_varents = if is_multi {
        let info_name = find_attribute_values(ast, "info")
            .last()
            .ok_or(syn::Error::new_spanned(ast, "please provide info"))?
            .to_owned();
        let info = Ident::new(&info_name, Span::call_site());

        quote! {
            enum #new_enum_identifier {
                #variants
                AnchorChange(iced_layershell::reexport::Anchor),
                LayerChange(iced_layershell::reexport::Layer),
                MarginChange((i32, i32, i32, i32)),
                SizeChange((u32, u32)),
                VirtualKeyboardPressed {
                    time: u32,
                    key: u32,
                },
                NewLayerShell((iced_layershell::actions::IcedNewMenuSettings, #info)),
                NewPopUp((iced_layershell::actions::IcedNewPopupSettings, #info)),
                NewMenu((iced_layershell::actions::IcedNewMenuSettings, #info)),
                RemoveWindow(iced::window::Id),
                ForgetLastOutput,
            }
            impl TryInto<iced_layershell::actions::LayershellCustomActionsWithIdAndInfo<#info>> for #new_enum_identifier {
                type Error = Self;

                fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActionsWithIdAndInfo<#info>, Self::Error> {
                    type InnerLayerAction = iced_layershell::actions::LayershellCustomActions<#info>;
                    match self {
                        Self::AnchorChange(anchor) => {
                            Ok(InnerLayerAction::AnchorChange(anchor))
                        }
                        Self::LayerChange(layer) => {
                            Ok(InnerLayerAction::LayerChange(layer))
                        }
                        Self::MarginChange(margin) => {
                            Ok(InnerLayerAction::MarginChange(margin))
                        }
                        Self::SizeChange(size) => {
                            Ok(InnerLayerAction::SizeChange(size))
                        }
                        Self::VirtualKeyboardPressed {
                            time,
                            key,
                        } => {

                            Ok(InnerLayerAction::VirtualKeyboardPressed {
                                time,
                                key
                            })
                        }
                        Self::NewLayerShell((settings, info)) => {
                            Ok(InnerLayerAction::NewLayerShell((settings, info)));
                        }
                        Self::NewPopUp(param) => Ok(InnerLayerAction::NewPopUp(param)),
                        Self::NewMenu(param) => Ok(InnerLayerAction::NewMenu(param)),
                        Self::RemoveWindow(id) => Ok(InnerLayerAction::RemoveWindow(id)),
                        Self::ForgetLastOutput => Ok(InnerLayerAction::ForgetLastOutput),
                        _ => Err(self)
                    }
                }
            }
        }
    } else {
        quote! {
            enum #new_enum_identifier {
                #variants
                AnchorChange(iced_layershell::reexport::Anchor),
                LayerChange(iced_layershell::reexport::Layer),
                MarginChange((i32, i32, i32, i32)),
                SizeChange((u32, u32)),
                VirtualKeyboardPressed {
                    time: u32,
                    key: u32,
                },

            }

            impl TryInto<iced_layershell::actions::LayershellCustomActions> for #new_enum_identifier {
                type Error = Self;
                fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActions, Self::Error> {
                    use iced_layershell::actions::LayershellCustomActions;
                    match self {
                        Self::AnchorChange(anchor) => {
                            Ok(LayershellCustomActions::AnchorChange(anchor))
                        },
                        Self::LayerChange(layer) => {
                            Ok(LayershellCustomActions::LayerChange(layer))
                        }
                        Self::MarginChange(margin) => {
                            Ok(LayershellCustomActions::MarginChange(margin))
                        }
                        Self::SizeChange(size) => {

                            Ok(LayershellCustomActions::SizeChange(size))
                        }
                        Self::VirtualKeyboardPressed {
                            time,
                            key,
                        } => {

                            Ok(LayershellCustomActions::VirtualKeyboardPressed {
                                time,
                                key
                            })
                        }
                        _ => Err(self)
                    }
                }
                // add code here
            }
        }
    };

    return Ok(TokenStream2::from(new_varents));
}

#[proc_macro_derive(LayerShellMessage, attributes(multi, info))]
pub fn layer_message_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match impl_layer_action_message(&ast) {
        Ok(ok) => ok.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
