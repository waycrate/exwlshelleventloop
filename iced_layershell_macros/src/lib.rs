use darling::{
    FromDeriveInput, FromMeta,
    ast::{Data, NestedMeta},
    util::{Flag, Ignored},
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

    let (additional_variants, impl_quote) = match is_multi {
        true => {
            let additional_variants = quote! {
                AnchorChange{id: iced::window::Id, anchor: iced_layershell::reexport::Anchor},
                SetInputRegion{ id: iced::window::Id, callback: iced_layershell::actions::ActionCallback },
                AnchorSizeChange{id: iced::window::Id, anchor:iced_layershell::reexport::Anchor, size: (u32, u32)},
                LayerChange{id: iced::window::Id, layer:iced_layershell::reexport::Layer},
                MarginChange{id: iced::window::Id, margin: (i32, i32, i32, i32)},
                SizeChange{id: iced::window::Id, size: (u32, u32)},
                ExclusiveZoneChange{id: iced::window::Id, zone_size: i32},
                VirtualKeyboardPressed {
                    time: u32,
                    key: u32,
                },
                NewLayerShell { settings: iced_layershell::reexport::NewLayerShellSettings, id: iced::window::Id },
                NewPopUp { settings: iced_layershell::actions::IcedNewPopupSettings, id: iced::window::Id },
                NewMenu { settings: iced_layershell::actions::IcedNewMenuSettings, id: iced::window::Id },
                NewInputPanel { settings: iced_layershell::reexport::NewInputPanelSettings, id: iced::window::Id },
                RemoveWindow(iced::window::Id),
                ForgetLastOutput,
            };

            let impl_quote = quote! {
                impl #impl_gen #ident #ty_gen #where_gen {
                    fn new_layershell(settings: iced_layershell::reexport::NewLayerShellSettings) -> (iced::window::Id, iced::Task<Self>) {
                        let id = iced::window::Id::unique();
                        (
                            id,
                            iced::Task::done(Self::NewLayerShell {
                                settings,
                                id,
                            })
                        )

                    }
                }
                impl #impl_gen TryInto<iced_layershell::actions::LayershellCustomActionsWithId> for #ident #ty_gen #where_gen {
                    type Error = Self;

                    fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActionsWithId, Self::Error> {
                        use iced_layershell::actions::LayershellCustomActions;
                        use iced_layershell::actions::LayershellCustomActionsWithId;

                        match self {
                            Self::SetInputRegion{ id, callback } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::SetInputRegion(callback))),
                            Self::AnchorChange { id, anchor } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::AnchorChange(anchor))),
                            Self::AnchorSizeChange { id, anchor, size } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::AnchorSizeChange(anchor, size))),
                            Self::LayerChange { id, layer } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::LayerChange(layer))),
                            Self::MarginChange { id, margin } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::MarginChange(margin))),
                            Self::SizeChange { id, size } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::SizeChange(size))),
                            Self::ExclusiveZoneChange { id, zone_size } => Ok(LayershellCustomActionsWithId::new(Some(id), LayershellCustomActions::ExclusiveZoneChange(zone_size))),
                            Self::VirtualKeyboardPressed { time, key } => Ok(LayershellCustomActionsWithId::new(
                                None,
                                LayershellCustomActions::VirtualKeyboardPressed { time, key })
                            ),
                            Self::NewLayerShell {settings, id } => Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::NewLayerShell { settings, id })),
                            Self::NewPopUp { settings, id } => Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::NewPopUp { settings, id })),
                            Self::NewMenu { settings, id } =>  Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::NewMenu {settings, id })),
                            Self::NewInputPanel {settings, id } => Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::NewInputPanel { settings, id })),
                            Self::RemoveWindow(id) => Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::RemoveWindow(id))),
                            Self::ForgetLastOutput => Ok(LayershellCustomActionsWithId::new(None, LayershellCustomActions::ForgetLastOutput)),
                            _ => Err(self)
                        }
                    }
                }
                impl #ident #ty_gen #where_gen {
                    fn layershell_open(settings: iced_layershell::reexport::NewLayerShellSettings) -> (iced::window::Id, Self) {
                        let id = iced::window::Id::unique();
                        (
                            id,
                            Self::NewLayerShell { settings, id }
                        )

                    }
                    fn popup_open(settings: iced_layershell::actions::IcedNewPopupSettings) -> (iced::window::Id, Self) {
                        let id = iced::window::Id::unique();
                        (
                            id,
                            Self::NewPopUp { settings, id }
                        )

                    }
                    fn menu_open(settings: iced_layershell::actions::IcedNewMenuSettings) -> (iced::window::Id, Self) {
                        let id = iced::window::Id::unique();
                        (
                            id,
                            Self::NewMenu { settings, id }
                        )

                    }
                }
            };
            (additional_variants, impl_quote)
        }
        false => {
            let additional_variants = quote! {
                AnchorChange(iced_layershell::reexport::Anchor),
                SetInputRegion(iced_layershell::actions::ActionCallback),
                AnchorSizeChange(iced_layershell::reexport::Anchor, (u32, u32)),
                LayerChange(iced_layershell::reexport::Layer),
                MarginChange((i32, i32, i32, i32)),
                SizeChange((u32, u32)),
                ExclusiveZoneChange(i32),
                VirtualKeyboardPressed {
                    time: u32,
                    key: u32,
                },
            };
            let impl_quote = quote! {
                impl #impl_gen TryInto<iced_layershell::actions::LayershellCustomActions> for #ident #ty_gen #where_gen {
                    type Error = Self;

                    fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActions, Self::Error> {
                        use iced_layershell::actions::LayershellCustomActions;

                        match self {
                            Self::SetInputRegion(callback) => Ok(LayershellCustomActions::SetInputRegion(callback)),
                            Self::AnchorChange(anchor) => Ok(LayershellCustomActions::AnchorChange(anchor)),
                            Self::AnchorSizeChange(anchor, size) => Ok(LayershellCustomActions::AnchorSizeChange(anchor, size)),
                            Self::LayerChange(layer) => Ok(LayershellCustomActions::LayerChange(layer)),

                            Self::MarginChange(margin) => Ok(LayershellCustomActions::MarginChange(margin)),
                            Self::SizeChange(size) => Ok(LayershellCustomActions::SizeChange(size)),
                            Self::ExclusiveZoneChange(zone_size) => Ok(LayershellCustomActions::ExclusiveZoneChange(zone_size)),
                            Self::VirtualKeyboardPressed { time, key } => Ok(LayershellCustomActions::VirtualKeyboardPressed {
                                time,
                                key
                            }),
                            _ => Err(self)
                        }
                    }
                }
            };

            (additional_variants, impl_quote)
        }
    };

    Ok(quote! {
        #(#attrs)*
        #vis enum #ident #ty_gen #where_gen {
            #(#variants,)*
            #additional_variants
        }

        #impl_quote
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
