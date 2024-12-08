use darling::{
    ast::{Data, NestedMeta},
    util::{Flag, Ignored},
    FromDeriveInput, FromMeta,
};
use proc_macro2::TokenStream as TokenStream2;
use syn::{DeriveInput, Generics, Ident, Variant, Visibility};

use quote::quote;

/// to_layer_message is to convert a normal enum to the enum usable in iced_layershell
/// It impl the try_into trait for the enum and make it can be convert to the actions in
/// layershell.
///
/// It will automatic add the fields which match the actions in iced_layershell
#[manyhow::manyhow]
#[proc_macro_attribute]
pub fn to_layer_message(attr: TokenStream2, input: TokenStream2) -> manyhow::Result<TokenStream2> {
    let meta = NestedMeta::parse_meta_list(attr)?;

    let ToLayerMessageAttr { multi } = ToLayerMessageAttr::from_list(&meta)?;

    let is_multi = multi.is_present();

    let derive_input = syn::parse2::<DeriveInput>(input)?;
    let attrs = &derive_input.attrs;
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
            let additional_variants = quote! {
                AnchorChange{id: iced::window::Id, anchor: iced_layershell::reexport::Anchor},
                AnchorSizeChange{id: iced::window::Id, anchor:iced_layershell::reexport::Anchor, size: (u32, u32)},
                LayerChange{id: iced::window::Id, layer:iced_layershell::reexport::Layer},
                MarginChange{id: iced::window::Id, margin: (i32, i32, i32, i32)},
                SizeChange{id: iced::window::Id, size: (u32, u32)},
                VirtualKeyboardPressed {
                    time: u32,
                    key: u32,
                },
                NewLayerShell { settings: iced_layershell::reexport::NewLayerShellSettings, id: iced::window::Id },
                NewPopUp { settings: iced_layershell::actions::IcedNewPopupSettings, id: iced::window::Id },
                NewMenu { settings: iced_layershell::actions::IcedNewMenuSettings, id: iced::window::Id },
                RemoveWindow(iced::window::Id),
                ForgetLastOutput,
            };
            let try_into_impl = quote! {
                impl #impl_gen TryInto<iced_layershell::actions::LayershellCustomActionsWithId> for #ident #ty_gen #where_gen {
                    type Error = Self;

                    fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActionsWithId, Self::Error> {
                        use iced_layershell::actions::LayershellCustomActions;
                        use iced_layershell::actions::LayershellCustomActionsWithId;

                        match self {
                            Self::AnchorChange { id, anchor } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::AnchorChange(anchor))),
                            Self::AnchorSizeChange { id, anchor, size } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::AnchorSizeChange(anchor, size))),
                            Self::LayerChange { id, layer } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::LayerChange(layer))),
                            Self::MarginChange { id, margin } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::MarginChange(margin))),
                            Self::SizeChange { id, size } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::SizeChange(size))),
                            Self::VirtualKeyboardPressed { time, key } => Ok(LayershellCustomActionsWithId::new(
                                None,
                                LayershellCustomActions::VirtualKeyboardPressed { time, key })
                            ),
                            Self::NewLayerShell {settings, id } => Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::NewLayerShell { settings, id })),
                            Self::NewPopUp { settings, id } => Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::NewPopUp { settings, id })),
                            Self::NewMenu { settings, id } =>  Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::NewMenu {settings, id })),
                            Self::RemoveWindow(id) => Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::RemoveWindow(id))),
                            Self::ForgetLastOutput => Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::ForgetLastOutput)),
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
                AnchorSizeChange(iced_layershell::reexport::Anchor, (u32, u32)),
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
                            Self::AnchorSizeChange(anchor, size) => Ok(LayershellCustomActions::AnchorSizeChange(anchor, size)),
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
}

#[derive(FromDeriveInput)]
#[darling(supports(enum_any))]
struct MessageEnum {
    vis: Visibility,
    ident: Ident,
    generics: Generics,
    data: Data<Variant, Ignored>,
}
