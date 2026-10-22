use darling::{
    FromDeriveInput, FromMeta,
    ast::{Data, NestedMeta},
    util::{Flag, Ignored},
};
use proc_macro2::TokenStream as TokenStream2;
use syn::{DeriveInput, Generics, Ident, Path, Variant, Visibility};

use quote::quote;

/// to_layer_message is to convert a normal enum to the enum usable in iced_exwlshell
/// It impl the try_into trait for the enum and make it can be convert to the actions in
/// layershell.
///
/// It will automatic add the fields which match the actions in iced_exwlshell
#[manyhow::manyhow]
#[proc_macro_attribute]
pub fn to_exwlshell_message(
    _attr: TokenStream2,
    input: TokenStream2,
) -> manyhow::Result<TokenStream2> {
    let derive_input = syn::parse2::<DeriveInput>(input)?;
    let attrs = &derive_input.attrs;
    let MessageEnum {
        vis,
        ident,
        generics,
        data,
    } = MessageEnum::from_derive_input(&derive_input)?;
    let wlshell_action: Path =
        syn::parse_quote!(iced_exwlshell::actions::ExWlShellCustomActionWithId);

    let (impl_gen, ty_gen, where_gen) = generics.split_for_impl();
    let variants = data.take_enum().unwrap();

    let (additional_variants, impl_quote) = {
        let additional_variants = quote! {
            /// Action, Anchor and size change
            LayoutChange{id: iced_exwlshell::reexport::IcedId, anchor: iced_exwlshell::reexport::Anchor, size: iced_exwlshell::reexport::LayerSize},
            /// Action, input region
            SetInputRegion{ id: iced_exwlshell::reexport::IcedId, callback: iced_exwlshell::actions::RegionActionCB },
            /// Action, layer change
            LayerChange{id: iced_exwlshell::reexport::IcedId, layer:iced_exwlshell::reexport::Layer},
            /// Action, margin change Margin: top, left, bottom, right
            MarginChange{id: iced_exwlshell::reexport::IcedId, margin: iced_exwlshell::reexport::Margin},
            BlurOptionChange{id: iced_exwlshell::reexport::IcedId, option: iced_exwlshell::reexport::BlurOption},
            /// Action, ExclusiveZone Change
            ExclusiveZoneChange{id: iced_exwlshell::reexport::IcedId, zone_size: i32},
            /// Action, KeyboardInteractivity change
            KeyboardInteractivityChange{id: iced_exwlshell::reexport::IcedId, keyboard_interactivity: iced_exwlshell::reexport::KeyboardInteractivity},
            /// Action, press vitral keyboard
            VirtualKeyboardPressed {
                key: u32,
            },
            /// Action request for new layershell
            NewLayerShell { settings: iced_exwlshell::reexport::NewLayerShellSettings, id: iced_exwlshell::reexport::IcedId },
            /// Action request for new base window
            NewBaseWindow { settings: iced_exwlshell::actions::IcedXdgWindowSettings, id: iced_exwlshell::reexport::IcedId },
            /// Action request for new base popup
            NewPopUp { settings: iced_exwlshell::actions::IcedNewPopupSettings, id: iced_exwlshell::reexport::IcedId },
            /// During move/resize of a mapped popup `settings.parent` is ignored. Can't be reparented as per spec
            PopUpReposition { settings: iced_exwlshell::actions::IcedNewPopupSettings, id: iced_exwlshell::reexport::IcedId },
            /// Action request for new menu
            NewMenu { settings: iced_exwlshell::actions::IcedNewMenuSettings, id: iced_exwlshell::reexport::IcedId },
            /// Action request for new input panel
            NewInputPanel { settings: iced_exwlshell::reexport::NewInputPanelSettings, id: iced_exwlshell::reexport::IcedId },
            /// Action, remove window
            RemoveWindow(iced_exwlshell::reexport::IcedId),
            /// Action, forget last outptu
            ForgetLastOutput,
            /// Action, Lock
            Lock,
            /// Action, UnLock
            UnLock
        };

        let impl_quote = quote! {
            impl #impl_gen #ident #ty_gen #where_gen {
                fn layershell_open(settings: iced_exwlshell::reexport::NewLayerShellSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                    let id = iced_exwlshell::reexport::IcedId::unique();
                    (
                        id,
                        iced_exwlshell::reexport::Task::done(Self::NewLayerShell { settings, id })
                    )

                }
                fn popup_open(settings: iced_exwlshell::actions::IcedNewPopupSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                    let id = iced_exwlshell::reexport::IcedId::unique();
                    (
                        id,
                        iced_exwlshell::reexport::Task::done(Self::NewPopUp { settings, id })
                    )

                }
                fn menu_open(settings: iced_exwlshell::actions::IcedNewMenuSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                    let id = iced_exwlshell::reexport::IcedId::unique();
                    (
                        id,
                        iced_exwlshell::reexport::Task::done(Self::NewMenu { settings, id })
                    )

                }
                fn base_window_open(settings: iced_exwlshell::actions::IcedXdgWindowSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                    let id = iced_exwlshell::reexport::IcedId::unique();
                    (
                        id,
                        iced_exwlshell::reexport::Task::done(Self::NewBaseWindow { settings, id })
                    )

                }
            }

            impl #impl_gen TryInto<#wlshell_action> for #ident #ty_gen #where_gen {
                type Error = Self;

                fn try_into(self) -> Result<iced_exwlshell::actions::ExWlShellCustomActionWithId, Self::Error> {
                    use iced_exwlshell::actions::ExWlShellCustomAction;
                    use iced_exwlshell::actions::ExWlShellCustomActionWithId;

                    match self {
                        Self::SetInputRegion{ id, callback } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::SetInputRegion(callback))),
                        Self::LayoutChange { id, anchor, size } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::LayoutChange { anchor, size })),
                        Self::LayerChange { id, layer } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::LayerChange(layer))),
                        Self::MarginChange { id, margin } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::MarginChange(margin))),
                        Self::ExclusiveZoneChange { id, zone_size } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::ExclusiveZoneChange(zone_size))),
                        Self::KeyboardInteractivityChange { id, keyboard_interactivity } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::KeyboardInteractivityChange(keyboard_interactivity))),
                        Self::VirtualKeyboardPressed { key } => Ok(ExWlShellCustomActionWithId::new(
                            None,
                            ExWlShellCustomAction::VirtualKeyboardPressed { key })
                        ),
                        Self::NewLayerShell {settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewLayerShell { settings, id })),
                        Self::NewBaseWindow {settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewBaseWindow { settings, id })),
                        Self::NewPopUp { settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewPopUp { settings, id })),
                        Self::PopUpReposition { settings, id } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::PopUpReposition { settings })),
                        Self::NewMenu { settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewMenu { settings, id })),
                        Self::NewInputPanel {settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewInputPanel { settings, id })),
                        Self::RemoveWindow(id) => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::RemoveWindow)),
                        Self::ForgetLastOutput => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::ForgetLastOutput)),
                        Self::BlurOptionChange {id, option} => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::BlurOptionChange(option))),
                        Self::Lock => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::Lock)),
                        Self::UnLock => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::UnLock)),
                        _ => Err(self)
                    }
                }
            }
        };
        (additional_variants, impl_quote)
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
#[manyhow::manyhow]
#[proc_macro_attribute]
pub fn to_sessionlock_message(
    _attr: TokenStream2,
    input: TokenStream2,
) -> manyhow::Result<TokenStream2> {
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

    let wlshell_action: Path =
        syn::parse_quote!(iced_exwlshell::actions::ExWlShellCustomActionWithId);

    let try_into = quote! {
        impl #impl_gen TryInto<#wlshell_action> for #ident #ty_gen #where_gen {
            type Error = Self;

            fn try_into(self) -> Result<#wlshell_action, Self::Error> {
                Err(self)
            }
        }
    };

    Ok(quote! {
        #(#attrs)*
        #vis enum #ident #ty_gen #where_gen {
            #(#variants,)*
        }

        #try_into
    })
}
#[derive(FromDeriveInput)]
#[darling(supports(enum_any))]
struct MessageEnum {
    vis: Visibility,
    ident: Ident,
    generics: Generics,
    data: Data<Variant, Ignored>,
}

#[derive(FromMeta)]
struct ToLayerMessageAttr {
    multi: Flag,
}

/// to_layer_message is to convert a normal enum to the enum usable in iced_exwlshell
/// It impl the try_into trait for the enum and make it can be convert to the actions in
/// layershell.
///
/// It will automatic add the fields which match the actions in iced_exwlshell
#[manyhow::manyhow]
#[proc_macro_attribute]
pub fn to_layer_message(attr: TokenStream2, input: TokenStream2) -> manyhow::Result<TokenStream2> {
    let meta = NestedMeta::parse_meta_list(attr)?;

    let ToLayerMessageAttr { multi } = ToLayerMessageAttr::from_list(&meta)?;

    let is_multi = multi.is_present();
    let wlshell_action: Path =
        syn::parse_quote!(iced_exwlshell::actions::ExWlShellCustomActionWithId);
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
                LayoutChange{id: iced_exwlshell::reexport::IcedId, anchor: iced_exwlshell::reexport::Anchor, size: iced_exwlshell::reexport::LayerSize},
                SetInputRegion{ id: iced_exwlshell::reexport::IcedId, callback: iced_exwlshell::actions::RegionActionCB },
                LayerChange{id: iced_exwlshell::reexport::IcedId, layer:iced_exwlshell::reexport::Layer},
                /// Margin: top, left, bottom, right
                MarginChange{id: iced_exwlshell::reexport::IcedId, margin: iced_exwlshell::reexport::Margin},
                BlurOptionChange{id: iced_exwlshell::reexport::IcedId, option: iced_exwlshell::reexport::BlurOption},
                ExclusiveZoneChange{id: iced_exwlshell::reexport::IcedId, zone_size: i32},
                KeyboardInteractivityChange{id: iced_exwlshell::reexport::IcedId, keyboard_interactivity: iced_exwlshell::reexport::KeyboardInteractivity},
                VirtualKeyboardPressed {
                    key: u32,
                },
                NewLayerShell { settings: iced_exwlshell::reexport::NewLayerShellSettings, id: iced_exwlshell::reexport::IcedId },
                NewBaseWindow { settings: iced_exwlshell::actions::IcedXdgWindowSettings, id: iced_exwlshell::reexport::IcedId },
                NewPopUp { settings: iced_exwlshell::actions::IcedNewPopupSettings, id: iced_exwlshell::reexport::IcedId },
                /// During move/resize of a mapped popup `settings.parent` is ignored. Can't be reparented as per spec
                PopUpReposition { settings: iced_exwlshell::actions::IcedNewPopupSettings, id: iced_exwlshell::reexport::IcedId },
                NewMenu { settings: iced_exwlshell::actions::IcedNewMenuSettings, id: iced_exwlshell::reexport::IcedId },
                NewInputPanel { settings: iced_exwlshell::reexport::NewInputPanelSettings, id: iced_exwlshell::reexport::IcedId },
                RemoveWindow(iced_exwlshell::reexport::IcedId),
                ForgetLastOutput,
            };

            let impl_quote = quote! {
                impl #impl_gen #ident #ty_gen #where_gen {
                    fn layershell_open(settings: iced_exwlshell::reexport::NewLayerShellSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                        let id = iced_exwlshell::reexport::IcedId::unique();
                        (
                            id,
                            iced_exwlshell::reexport::Task::done(Self::NewLayerShell { settings, id })
                        )

                    }
                    fn popup_open(settings: iced_exwlshell::actions::IcedNewPopupSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                        let id = iced_exwlshell::reexport::IcedId::unique();
                        (
                            id,
                            iced_exwlshell::reexport::Task::done(Self::NewPopUp { settings, id })
                        )

                    }
                    fn menu_open(settings: iced_exwlshell::actions::IcedNewMenuSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                        let id = iced_exwlshell::reexport::IcedId::unique();
                        (
                            id,
                            iced_exwlshell::reexport::Task::done(Self::NewMenu { settings, id })
                        )

                    }
                    fn base_window_open(settings: iced_exwlshell::actions::IcedXdgWindowSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                        let id = iced_exwlshell::reexport::IcedId::unique();
                        (
                            id,
                            iced_exwlshell::reexport::Task::done(Self::NewBaseWindow { settings, id })
                        )

                    }
                }
                impl #impl_gen TryInto<#wlshell_action> for #ident #ty_gen #where_gen {
                    type Error = Self;

                    fn try_into(self) -> Result<#wlshell_action, Self::Error> {
                        use iced_exwlshell::actions::ExWlShellCustomAction;
                        use iced_exwlshell::actions::ExWlShellCustomActionWithId;

                        match self {
                            Self::SetInputRegion{ id, callback } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::SetInputRegion(callback))),
                            Self::LayoutChange { id, anchor, size } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::LayoutChange{ anchor, size })),
                            Self::LayerChange { id, layer } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::LayerChange(layer))),
                            Self::MarginChange { id, margin } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::MarginChange(margin))),
                            Self::ExclusiveZoneChange { id, zone_size } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::ExclusiveZoneChange(zone_size))),
                            Self::KeyboardInteractivityChange { id, keyboard_interactivity } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::KeyboardInteractivityChange(keyboard_interactivity))),
                            Self::VirtualKeyboardPressed { key } => Ok(ExWlShellCustomActionWithId::new(
                                None,
                                ExWlShellCustomAction::VirtualKeyboardPressed { key })
                            ),
                            Self::NewLayerShell {settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewLayerShell { settings, id })),
                            Self::NewBaseWindow {settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewBaseWindow { settings, id })),
                            Self::NewPopUp { settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewPopUp { settings, id })),
                            Self::PopUpReposition { settings, id } => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::PopUpReposition { settings })),
                            Self::NewMenu { settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewMenu { settings, id })),
                            Self::NewInputPanel {settings, id } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::NewInputPanel { settings, id })),
                            Self::RemoveWindow(id) => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::RemoveWindow)),
                            Self::ForgetLastOutput => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::ForgetLastOutput)),
                            Self::BlurOptionChange {id, option} => Ok(ExWlShellCustomActionWithId::new(Some(id), ExWlShellCustomAction::BlurOptionChange(option))),
                            _ => Err(self)
                        }
                    }
                }
            };
            (additional_variants, impl_quote)
        }
        false => {
            let additional_variants = quote! {
                LayoutChange { anchor: iced_exwlshell::reexport::Anchor, size: iced_exwlshell::reexport::LayerSize },
                SetInputRegion(iced_exwlshell::actions::RegionActionCB),
                LayerChange(iced_exwlshell::reexport::Layer),
                /// Margin: top, right, bottom, left
                MarginChange(iced_exwlshell::reexport::Margin),
                ExclusiveZoneChange(i32),
                KeyboardInteractivityChange(iced_exwlshell::reexport::KeyboardInteractivity),
                VirtualKeyboardPressed {
                    key: u32,
                },
                BlurOptionChange(iced_exwlshell::reexport::BlurOption),
            };
            let impl_quote = quote! {
                impl #impl_gen TryInto<#wlshell_action> for #ident #ty_gen #where_gen {
                    type Error = Self;

                    fn try_into(self) -> Result<iced_exwlshell::actions::ExWlShellCustomActionWithId, Self::Error> {
                        use iced_exwlshell::actions::ExWlShellCustomAction;
                        use iced_exwlshell::actions::ExWlShellCustomActionWithId;

                        match self {
                            Self::SetInputRegion(callback) => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::SetInputRegion(callback))),
                            Self::LayoutChange { anchor, size } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::LayoutChange { anchor, size })),
                            Self::LayerChange(layer) => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::LayerChange(layer))),

                            Self::MarginChange(margin) => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::MarginChange(margin))),
                            Self::ExclusiveZoneChange(zone_size) => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::ExclusiveZoneChange(zone_size))),
                            Self::KeyboardInteractivityChange(keyboard_interactivity) => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::KeyboardInteractivityChange(keyboard_interactivity))),
                            Self::VirtualKeyboardPressed { key } => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::VirtualKeyboardPressed {
                                key
                            })),

                            Self::BlurOptionChange(option) => Ok(ExWlShellCustomActionWithId::new(None, ExWlShellCustomAction::BlurOptionChange(option))),
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
