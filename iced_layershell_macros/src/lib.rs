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
                AnchorChange{id: iced_layershell::reexport::IcedId, anchor: iced_layershell::reexport::Anchor},
                SetInputRegion{ id: iced_layershell::reexport::IcedId, callback: iced_layershell::actions::ActionCallback },
                AnchorSizeChange{id: iced_layershell::reexport::IcedId, anchor:iced_layershell::reexport::Anchor, size: (u32, u32)},
                LayerChange{id: iced_layershell::reexport::IcedId, layer:iced_layershell::reexport::Layer},
                /// Margin: top, left, bottom, right
                MarginChange{id: iced_layershell::reexport::IcedId, margin: (i32, i32, i32, i32)},
                SizeChange{id: iced_layershell::reexport::IcedId, size: (u32, u32)},
                ExclusiveZoneChange{id: iced_layershell::reexport::IcedId, zone_size: i32},
                VirtualKeyboardPressed {
                    time: u32,
                    key: u32,
                },
                NewLayerShell { settings: iced_layershell::reexport::NewLayerShellSettings, id: iced_layershell::reexport::IcedId },
                NewBaseWindow { settings: iced_layershell::actions::IcedXdgWindowSettings, id: iced_layershell::reexport::IcedId },
                NewPopUp { settings: iced_layershell::actions::IcedNewPopupSettings, id: iced_layershell::reexport::IcedId },
                NewMenu { settings: iced_layershell::actions::IcedNewMenuSettings, id: iced_layershell::reexport::IcedId },
                NewInputPanel { settings: iced_layershell::reexport::NewInputPanelSettings, id: iced_layershell::reexport::IcedId },
                RemoveWindow(iced_layershell::reexport::IcedId),
                ForgetLastOutput,
            };

            let impl_quote = quote! {
                impl #impl_gen #ident #ty_gen #where_gen {
                    fn layershell_open(settings: iced_layershell::reexport::NewLayerShellSettings) -> (iced_layershell::reexport::IcedId, iced_layershell::reexport::Task<Self>) {
                        let id = iced_layershell::reexport::IcedId::unique();
                        (
                            id,
                            iced_layershell::reexport::Task::done(Self::NewLayerShell { settings, id })
                        )

                    }
                    fn popup_open(settings: iced_layershell::actions::IcedNewPopupSettings) -> (iced_layershell::reexport::IcedId, iced_layershell::reexport::Task<Self>) {
                        let id = iced_layershell::reexport::IcedId::unique();
                        (
                            id,
                            iced_layershell::reexport::Task::done(Self::NewPopUp { settings, id })
                        )

                    }
                    fn base_window_open(settings: iced_layershell::actions::IcedXdgWindowSettings) -> (iced_layershell::reexport::IcedId, iced_layershell::reexport::Task<Self>) {
                        let id = iced_layershell::reexport::IcedId::unique();
                        (
                            id,
                            iced_layershell::reexport::Task::done(Self::NewBaseWindow { settings, id })
                        )

                    }
                    fn menu_open(settings: iced_layershell::actions::IcedNewMenuSettings) -> (iced_layershell::reexport::IcedId, iced_layershell::reexport::Task<Self>) {
                        let id = iced_layershell::reexport::IcedId::unique();
                        (
                            id,
                            iced_layershell::reexport::Task::done(Self::NewMenu { settings, id })
                        )

                    }
                }
                impl #impl_gen TryInto<iced_layershell::actions::LayershellCustomActionWithId> for #ident #ty_gen #where_gen {
                    type Error = Self;

                    fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActionWithId, Self::Error> {
                        use iced_layershell::actions::LayershellCustomAction;
                        use iced_layershell::actions::LayershellCustomActionWithId;

                        match self {
                            Self::SetInputRegion{ id, callback } => Ok(LayershellCustomActionWithId::new(Some(id), LayershellCustomAction::SetInputRegion(callback))),
                            Self::AnchorChange { id, anchor } => Ok(LayershellCustomActionWithId::new(Some(id), LayershellCustomAction::AnchorChange(anchor))),
                            Self::AnchorSizeChange { id, anchor, size } => Ok(LayershellCustomActionWithId::new(Some(id), LayershellCustomAction::AnchorSizeChange(anchor, size))),
                            Self::LayerChange { id, layer } => Ok(LayershellCustomActionWithId::new(Some(id), LayershellCustomAction::LayerChange(layer))),
                            Self::MarginChange { id, margin } => Ok(LayershellCustomActionWithId::new(Some(id), LayershellCustomAction::MarginChange(margin))),
                            Self::SizeChange { id, size } => Ok(LayershellCustomActionWithId::new(Some(id), LayershellCustomAction::SizeChange(size))),
                            Self::ExclusiveZoneChange { id, zone_size } => Ok(LayershellCustomActionWithId::new(Some(id), LayershellCustomAction::ExclusiveZoneChange(zone_size))),
                            Self::VirtualKeyboardPressed { time, key } => Ok(LayershellCustomActionWithId::new(
                                None,
                                LayershellCustomAction::VirtualKeyboardPressed { time, key })
                            ),
                            Self::NewLayerShell {settings, id } => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::NewLayerShell { settings, id })),
                            Self::NewBaseWindow {settings, id } => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::NewBaseWindow { settings, id })),
                            Self::NewPopUp { settings, id } => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::NewPopUp { settings, id })),
                            Self::NewMenu { settings, id } =>  Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::NewMenu {settings, id })),
                            Self::NewInputPanel {settings, id } => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::NewInputPanel { settings, id })),
                            Self::RemoveWindow(id) => Ok(LayershellCustomActionWithId::new(Some(id), LayershellCustomAction::RemoveWindow)),
                            Self::ForgetLastOutput => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::ForgetLastOutput)),
                            _ => Err(self)
                        }
                    }
                }
            };
            (additional_variants, impl_quote)
        }
        false => {
            let additional_variants = quote! {
                AnchorChange(iced_layershell::reexport::Anchor),
                SetInputRegion(iced_layershell::actions::ActionCallback),
                // Ancher and Size (width, height)
                AnchorSizeChange(iced_layershell::reexport::Anchor, (u32, u32)),
                LayerChange(iced_layershell::reexport::Layer),
                /// Margin: top, left, bottom, right
                MarginChange((i32, i32, i32, i32)),
                SizeChange((u32, u32)),
                ExclusiveZoneChange(i32),
                VirtualKeyboardPressed {
                    time: u32,
                    key: u32,
                },
            };
            let impl_quote = quote! {
                impl #impl_gen TryInto<iced_layershell::actions::LayershellCustomActionWithId> for #ident #ty_gen #where_gen {
                    type Error = Self;

                    fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActionWithId, Self::Error> {
                        use iced_layershell::actions::LayershellCustomAction;
                        use iced_layershell::actions::LayershellCustomActionWithId;

                        match self {
                            Self::SetInputRegion(callback) => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::SetInputRegion(callback))),
                            Self::AnchorChange(anchor) => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::AnchorChange(anchor))),
                            Self::AnchorSizeChange(anchor, size) => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::AnchorSizeChange(anchor, size))),
                            Self::LayerChange(layer) => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::LayerChange(layer))),

                            Self::MarginChange(margin) => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::MarginChange(margin))),
                            Self::SizeChange(size) => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::SizeChange(size))),
                            Self::ExclusiveZoneChange(zone_size) => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::ExclusiveZoneChange(zone_size))),
                            Self::VirtualKeyboardPressed { time, key } => Ok(LayershellCustomActionWithId::new(None, LayershellCustomAction::VirtualKeyboardPressed {
                                time,
                                key
                            })),
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
