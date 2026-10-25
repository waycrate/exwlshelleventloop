use std::any::Any;

use exwlshellev::{blur::BlurRegion, input_rectangle::InputRectangle};
use iced_core::{
    Rectangle,
    widget::{Id, Operation},
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlurRegions(pub Vec<BlurRegion>);

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InputRects(pub Vec<Rectangle>);

#[derive(Default)]
pub(crate) struct SurfaceRegionsCollector {
    pub(crate) blur: Option<Vec<BlurRegion>>,
    pub(crate) input: Option<Vec<InputRectangle>>,
}

#[allow(clippy::cast_possible_truncation)]
fn covering_input_rectangle(rect: &Rectangle) -> InputRectangle {
    InputRectangle {
        x: rect.x as i32,
        y: rect.y as i32,
        width: rect.width.ceil() as i32,
        height: rect.height.ceil() as i32,
    }
}

impl Operation for SurfaceRegionsCollector {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn custom(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Any) {
        if let Some(BlurRegions(regions)) = state.downcast_ref::<BlurRegions>() {
            self.blur.get_or_insert_default().extend_from_slice(regions);
        }
        if let Some(InputRects(rects)) = state.downcast_ref::<InputRects>() {
            self.input.get_or_insert_default().extend(
                rects
                    .iter()
                    .filter(|rect| rect.width > 0.0 && rect.height > 0.0)
                    .map(covering_input_rectangle),
            );
        }
    }
}
