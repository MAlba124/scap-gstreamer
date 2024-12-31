use gst::prelude::*;
use gst::MessageView;

fn main() {
    gst::init().unwrap();
    scapgst::plugin_register_static().unwrap();

    let pipeline = gst::parse::launch("scapsrc ! videoconvert ! autovideosink").unwrap();
    let bus = pipeline.bus().unwrap();

    pipeline.set_state(gst::State::Playing).unwrap();

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        match msg.view() {
            MessageView::Eos(_) => break,
            MessageView::Error(e) => {
                eprintln!("Error {:?} {} {:?}", e.src().map(|s| s.path_string()), e.error(), e.debug());
                break;
            }
            _ => {},
        }
    }

    pipeline.set_state(gst::State::Null).unwrap();
}

