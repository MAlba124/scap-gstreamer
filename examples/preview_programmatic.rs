// This example shows how to build the same pipeline used in `preview` but
// by manually creating and linking elements.

use gst::prelude::*;
use gst::MessageView;

fn main() {
    gst::init().unwrap();
    scapgst::plugin_register_static().unwrap();

    let pipeline = gst::Pipeline::default();

    let scapsrc = gst::ElementFactory::make("scapsrc").build().unwrap();

    scapsrc.set_property("show-cursor", true);
    scapsrc.set_property("fps", 10 as u32);

    let videoconvert = gst::ElementFactory::make("videoconvert").build().unwrap();
    let autovideosink = gst::ElementFactory::make("autovideosink").build().unwrap();

    pipeline
        .add_many([&scapsrc, &videoconvert, &autovideosink])
        .unwrap();
    gst::Element::link_many([&scapsrc, &videoconvert, &autovideosink]).unwrap();

    let bus = pipeline.bus().unwrap();

    pipeline.set_state(gst::State::Playing).unwrap();

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        match msg.view() {
            MessageView::Eos(_) => break,
            MessageView::Error(e) => {
                eprintln!(
                    "Error {:?} {} {:?}",
                    e.src().map(|s| s.path_string()),
                    e.error(),
                    e.debug()
                );
                break;
            }
            _ => {}
        }
    }

    pipeline.set_state(gst::State::Null).unwrap();
}
