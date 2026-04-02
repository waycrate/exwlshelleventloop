use darling::{FromDeriveInput, ast::Data, util::Ignored};
use proc_macro2::TokenStream as TokenStream2;
use syn::{DeriveInput, Generics, Ident, Variant, Visibility};

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

    let (impl_gen, ty_gen, where_gen) = generics.split_for_impl();
    let variants = data.take_enum().unwrap();

    let (additional_variants, impl_quote) = {
        let additional_variants = quote! {
            NewShell(iced_exwlshell::NewShellInfo),
            AnchorChange{id: iced_exwlshell::reexport::IcedId, anchor: iced_exwlshell::reexport::Anchor},
            SetInputRegion{ id: iced_exwlshell::reexport::IcedId, callback: iced_exwlshell::actions::ActionCallback },
            AnchorSizeChange{id: iced_exwlshell::reexport::IcedId, anchor:iced_exwlshell::reexport::Anchor, size: (u32, u32)},
            LayerChange{id: iced_exwlshell::reexport::IcedId, layer:iced_exwlshell::reexport::Layer},
            /// Margin: top, left, bottom, right
            MarginChange{id: iced_exwlshell::reexport::IcedId, margin: (i32, i32, i32, i32)},
            SizeChange{id: iced_exwlshell::reexport::IcedId, size: (u32, u32)},
            ExclusiveZoneChange{id: iced_exwlshell::reexport::IcedId, zone_size: i32},
            KeyboardInteractivityChange{id: iced_exwlshell::reexport::IcedId, keyboard_interactivity: iced_exwlshell::reexport::KeyboardInteractivity},
            VirtualKeyboardPressed {
                time: u32,
                key: u32,
            },
            NewLayerShell { settings: iced_exwlshell::reexport::NewLayerShellSettings, id: iced_exwlshell::reexport::IcedId },
            NewBaseWindow { settings: iced_exwlshell::actions::IcedXdgWindowSettings, id: iced_exwlshell::reexport::IcedId },
            NewPopUp { settings: iced_exwlshell::actions::IcedNewPopupSettings, id: iced_exwlshell::reexport::IcedId },
            NewMenu { settings: iced_exwlshell::actions::IcedNewMenuSettings, id: iced_exwlshell::reexport::IcedId },
            NewInputPanel { settings: iced_exwlshell::reexport::NewInputPanelSettings, id: iced_exwlshell::reexport::IcedId },
            RemoveWindow(iced_exwlshell::reexport::IcedId),
            ForgetLastOutput,
            Lock,
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
                fn base_window_open(settings: iced_exwlshell::actions::IcedXdgWindowSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                    let id = iced_exwlshell::reexport::IcedId::unique();
                    (
                        id,
                        iced_exwlshell::reexport::Task::done(Self::NewBaseWindow { settings, id })
                    )

                }
                fn menu_open(settings: iced_exwlshell::actions::IcedNewMenuSettings) -> (iced_exwlshell::reexport::IcedId, iced_exwlshell::reexport::Task<Self>) {
                    let id = iced_exwlshell::reexport::IcedId::unique();
                    (
                        id,
                        iced_exwlshell::reexport::Task::done(Self::NewMenu { settings, id })
                    )

                }
            }

            impl #impl_gen iced_exwlshell::FromShellInfo for #ident #ty_gen #where_gen {
                fn get(shell: iced_exwlshell::NewShellInfo) -> Self {
                    Self::NewShell(shell)
                }
            }

            impl #impl_gen TryInto<iced_exwlshell::actions::ExwlShellCustomActionWithId> for #ident #ty_gen #where_gen {
                type Error = Self;

                fn try_into(self) -> Result<iced_exwlshell::actions::ExwlShellCustomActionWithId, Self::Error> {
                    use iced_exwlshell::actions::ExwlShellCustomAction;
                    use iced_exwlshell::actions::ExwlShellCustomActionWithId;

                    match self {
                        Self::SetInputRegion{ id, callback } => Ok(ExwlShellCustomActionWithId::new(Some(id), ExwlShellCustomAction::SetInputRegion(callback))),
                        Self::AnchorChange { id, anchor } => Ok(ExwlShellCustomActionWithId::new(Some(id), ExwlShellCustomAction::AnchorChange(anchor))),
                        Self::AnchorSizeChange { id, anchor, size } => Ok(ExwlShellCustomActionWithId::new(Some(id), ExwlShellCustomAction::AnchorSizeChange(anchor, size))),
                        Self::LayerChange { id, layer } => Ok(ExwlShellCustomActionWithId::new(Some(id), ExwlShellCustomAction::LayerChange(layer))),
                        Self::MarginChange { id, margin } => Ok(ExwlShellCustomActionWithId::new(Some(id), ExwlShellCustomAction::MarginChange(margin))),
                        Self::SizeChange { id, size } => Ok(ExwlShellCustomActionWithId::new(Some(id), ExwlShellCustomAction::SizeChange(size))),
                        Self::ExclusiveZoneChange { id, zone_size } => Ok(ExwlShellCustomActionWithId::new(Some(id), ExwlShellCustomAction::ExclusiveZoneChange(zone_size))),
                        Self::KeyboardInteractivityChange { id, keyboard_interactivity } => Ok(ExwlShellCustomActionWithId::new(Some(id), ExwlShellCustomAction::KeyboardInteractivityChange(keyboard_interactivity))),
                        Self::VirtualKeyboardPressed { time, key } => Ok(ExwlShellCustomActionWithId::new(
                            None,
                            ExwlShellCustomAction::VirtualKeyboardPressed { time, key })
                        ),
                        Self::NewLayerShell {settings, id } => Ok(ExwlShellCustomActionWithId::new(None, ExwlShellCustomAction::NewLayerShell { settings, id })),
                        Self::NewBaseWindow {settings, id } => Ok(ExwlShellCustomActionWithId::new(None, ExwlShellCustomAction::NewBaseWindow { settings, id })),
                        Self::NewPopUp { settings, id } => Ok(ExwlShellCustomActionWithId::new(None, ExwlShellCustomAction::NewPopUp { settings, id })),
                        Self::NewMenu { settings, id } =>  Ok(ExwlShellCustomActionWithId::new(None, ExwlShellCustomAction::NewMenu {settings, id })),
                        Self::NewInputPanel {settings, id } => Ok(ExwlShellCustomActionWithId::new(None, ExwlShellCustomAction::NewInputPanel { settings, id })),
                        Self::RemoveWindow(id) => Ok(ExwlShellCustomActionWithId::new(Some(id), ExwlShellCustomAction::RemoveWindow)),
                        Self::ForgetLastOutput => Ok(ExwlShellCustomActionWithId::new(None, ExwlShellCustomAction::ForgetLastOutput)),
                        Self::Lock => Ok(ExwlShellCustomActionWithId::new(None, ExwlShellCustomAction::Lock)),
                        Self::UnLock => Ok(ExwlShellCustomActionWithId::new(None, ExwlShellCustomAction::UnLock)),
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

#[derive(FromDeriveInput)]
#[darling(supports(enum_any))]
struct MessageEnum {
    vis: Visibility,
    ident: Ident,
    generics: Generics,
    data: Data<Variant, Ignored>,
}
