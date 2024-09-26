use darling::{
    ast::{Data, NestedMeta},
    util::{Flag, Ignored},
    FromDeriveInput, FromMeta,
};
use proc_macro2::{Span, TokenStream as TokenStream2};
use syn::{
    punctuated::Punctuated, DeriveInput, Generics, Ident, LitStr, Meta, Token, Variant, Visibility,
};

use quote::quote;

#[manyhow::manyhow]
#[proc_macro_attribute]
pub fn to_layer_message(attr: TokenStream2, input: TokenStream2) -> manyhow::Result<TokenStream2> {
    let meta = NestedMeta::parse_meta_list(attr)?;

    let ToLayerMessageAttr {
        multi,
        info_name,
        attrs,
    } = ToLayerMessageAttr::from_list(&meta)?;

    let is_multi = multi.is_present();
    let attrs = attrs.into_iter().map(|meta| quote!(#[#meta]));

    let derive_input = syn::parse2::<DeriveInput>(input)?;
    let MessageEnum {
        vis,
        ident,
        generics,
        data,
    } = MessageEnum::from_derive_input(&derive_input)?;

    let (impl_gen, ty_gen, where_gen) = generics.split_for_impl();
    let variants = data.take_enum().unwrap();

    let (additional_variants, try_into_impl) = match is_multi {
        true => {
            let info_name = info_name.expect("Should set the info_name").value();
            let info = Ident::new(&info_name, Span::call_site());

            let additional_variants = quote! {
                AnchorChange{id: iced::window::Id, anchor: iced_layershell::reexport::Anchor},
                LayerChange{id: iced::window::Id, layer:iced_layershell::reexport::Layer},
                MarginChange{id: iced::window::Id, margin: (i32, i32, i32, i32)},
                SizeChange{id: iced::window::Id, size: (u32, u32)},
                VirtualKeyboardPressed {
                    time: u32,
                    key: u32,
                },
                NewLayerShell { settings: iced_layershell::reexport::NewLayerShellSettings, info: #info },
                NewPopUp { settings: iced_layershell::actions::IcedNewPopupSettings, info: #info },
                NewMenu { settings: iced_layershell::actions::IcedNewMenuSettings, info: #info },
                RemoveWindow(iced::window::Id),
                ForgetLastOutput,
            };

            let try_into_impl = quote! {
                impl #impl_gen TryInto<iced_layershell::actions::LayershellCustomActionsWithIdAndInfo<#info>> for #ident #ty_gen #where_gen {
                    type Error = Self;

                    fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActionsWithIdAndInfo<#info>, Self::Error> {
                        type InnerLayerActionId = iced_layershell::actions::LayershellCustomActionsWithIdAndInfo<#info>;
                        type InnerLayerAction = iced_layershell::actions::LayershellCustomActionsWithInfo<#info>;

                        match self {
                            Self::AnchorChange { id, anchor } => Ok(InnerLayerActionId::new(Some(id), InnerLayerAction::AnchorChange(anchor))),
                            Self::LayerChange { id, layer } => Ok(InnerLayerActionId::new(Some(id), InnerLayerAction::LayerChange(layer))),
                            Self::MarginChange { id, margin } => Ok(InnerLayerActionId::new(Some(id), InnerLayerAction::MarginChange(margin))),
                            Self::SizeChange { id, size } => Ok(InnerLayerActionId::new(Some(id), InnerLayerAction::SizeChange(size))),
                            Self::VirtualKeyboardPressed { time, key } => Ok(InnerLayerActionId::new(
                                None,
                                InnerLayerAction::VirtualKeyboardPressed { time, key })
                            ),
                            Self::NewLayerShell {settings, info } => Ok(InnerLayerActionId::new(None, InnerLayerAction::NewLayerShell((settings, info)))),
                            Self::NewPopUp { settings, info } => Ok(InnerLayerActionId::new(None, InnerLayerAction::NewPopUp((settings, info)))),
                            Self::NewMenu { settings, info } =>  Ok(InnerLayerActionId::new(None, InnerLayerAction::NewMenu((settings, info)))),
                            Self::RemoveWindow(id) => Ok(InnerLayerActionId::new(None, InnerLayerAction::RemoveWindow(id))),
                            Self::ForgetLastOutput => Ok(InnerLayerActionId::new(None, InnerLayerAction::ForgetLastOutput)),
                            _ => Err(self)
                        }
                    }
                }
            };

            (additional_variants, try_into_impl)
        }
        false => {
            let additional_variants = quote! {
                AnchorChange(iced_layershell::reexport::Anchor),
                LayerChange(iced_layershell::reexport::Layer),
                MarginChange((i32, i32, i32, i32)),
                SizeChange((u32, u32)),
                VirtualKeyboardPressed {
                    time: u32,
                    key: u32,
                },
            };
            let try_into_impl = quote! {
                impl #impl_gen TryInto<iced_layershell::actions::LayershellCustomActions> for #ident #ty_gen #where_gen {
                    type Error = Self;

                    fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActions, Self::Error> {
                        use iced_layershell::actions::LayershellCustomActions;

                        match self {
                            Self::AnchorChange(anchor) => Ok(LayershellCustomActions::AnchorChange(anchor)),
                            Self::LayerChange(layer) => Ok(LayershellCustomActions::LayerChange(layer)),
                            Self::MarginChange(margin) => Ok(LayershellCustomActions::MarginChange(margin)),
                            Self::SizeChange(size) => Ok(LayershellCustomActions::SizeChange(size)),
                            Self::VirtualKeyboardPressed { time, key } => Ok(LayershellCustomActions::VirtualKeyboardPressed {
                                time,
                                key
                            }),
                            _ => Err(self)
                        }
                    }
                }
            };

            (additional_variants, try_into_impl)
        }
    };

    Ok(quote! {
        #(#attrs)*
        #vis enum #ident #ty_gen #where_gen {
            #(#variants,)*
            #additional_variants
        }

        #try_into_impl
    })
}

#[derive(FromMeta)]
struct ToLayerMessageAttr {
    multi: Flag,
    info_name: Option<LitStr>,

    #[darling(default)]
    attrs: Punctuated<Meta, Token![|]>,
}

#[derive(FromDeriveInput)]
#[darling(supports(enum_any))]
struct MessageEnum {
    vis: Visibility,
    ident: Ident,
    generics: Generics,
    data: Data<Variant, Ignored>,
}
