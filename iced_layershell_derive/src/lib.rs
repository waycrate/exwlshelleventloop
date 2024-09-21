extern crate proc_macro;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use syn::{parse_macro_input, ItemEnum, LitStr};

use quote::quote;

#[proc_macro_attribute]
pub fn layer_message_attribute(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut is_multi = false;
    let mut info_name: Option<LitStr> = None;
    let mut derives: Option<LitStr> = None;
    let tea_parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("multi") {
            is_multi = true;
            Ok(())
        } else if meta.path.is_ident("info_name") {
            info_name = Some(meta.value()?.parse()?);
            Ok(())
        } else if meta.path.is_ident("derives") {
            derives = Some(meta.value()?.parse()?);
            Ok(())
        } else {
            Err(meta.error("unsupported tea property"))
        }
    });
    let input_enum = parse_macro_input!(input as ItemEnum);
    parse_macro_input!(attr with tea_parser);
    let mut derive_part = vec![];
    if let Some(derives) = derives {
        let tmpval = derives.value();
        let val: Vec<&str> = tmpval.split(' ').collect();
        for der in val.iter() {
            let iden_tmp = Ident::new(der, Span::call_site());
            derive_part.push(quote! {
                #[derive(#iden_tmp)]
            });
        }
    }
    // Extract the enum name
    let enum_name = &input_enum.ident;
    let variants = &input_enum.variants;
    let new_varents = if is_multi {
        let info_name = info_name.expect("Should set the infoName").value();
        let info = Ident::new(&info_name, Span::call_site());

        quote! {
            #(#derive_part)*
            enum #enum_name {
                #variants
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
            }
            impl TryInto<iced_layershell::actions::LayershellCustomActionsWithIdAndInfo<#info>> for #enum_name {
                type Error = Self;

                fn try_into(self) -> Result<iced_layershell::actions::LayershellCustomActionsWithIdAndInfo<#info>, Self::Error> {
                    type InnerLayerActionId = iced_layershell::actions::LayershellCustomActionsWithIdAndInfo<#info>;
                    type InnerLayerAction = LayershellCustomActionsWithInfo<#info>;
                    match self {
                        Self::AnchorChange{id, anchor} => {
                            Ok(InnerLayerActionId::new(Some(id), InnerLayerAction::AnchorChange(anchor)))
                        }
                        Self::LayerChange{id, layer} => {
                            Ok(InnerLayerActionId::new(Some(id), InnerLayerAction::LayerChange(layer)))
                        }
                        Self::MarginChange{id, margin} => {
                            Ok(InnerLayerActionId::new(Some(id), InnerLayerAction::MarginChange(margin)))
                        }
                        Self::SizeChange {id,size} => {
                            Ok(InnerLayerActionId::new(Some(id), InnerLayerAction::SizeChange(size)))
                        }
                        Self::VirtualKeyboardPressed {
                            time,
                            key,
                        } => {
                            Ok(InnerLayerActionId::new(None, InnerLayerAction::VirtualKeyboardPressed {
                                time,
                                key
                            }))
                        }
                        Self::NewLayerShell {settings, info } => {
                            Ok(InnerLayerActionId::new(None, InnerLayerAction::NewLayerShell((settings, info))))
                        }
                        Self::NewPopUp { settings, info } =>  Ok(InnerLayerActionId::new(None, InnerLayerAction::NewPopUp((settings, info)))),
                        Self::NewMenu { settings, info } =>   Ok(InnerLayerActionId::new(None, InnerLayerAction::NewMenu((settings, info)))),
                        Self::RemoveWindow(id) => Ok(InnerLayerActionId::new(None, InnerLayerAction::RemoveWindow(id))),
                        Self::ForgetLastOutput => Ok(InnerLayerActionId::new(None, InnerLayerAction::ForgetLastOutput)),
                        _ => Err(self)
                    }
                }
            }
        }
    } else {
        quote! {
            #(#derive_part)*
            enum #enum_name {
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

            impl TryInto<iced_layershell::actions::LayershellCustomActions> for #enum_name {
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

    TokenStream::from(new_varents)
}
