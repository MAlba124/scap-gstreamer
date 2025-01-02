use gst::MessageView;
use gst::prelude::*;

fn main() {
    gst::init().unwrap();
    scapgst::plugin_register_static().unwrap();

    let pipeline = gst::parse::launch(
        "scapsrc ! videoconvert ! x264enc ! matroskamux ! filesink location=screencast.mkv",
    )
    .unwrap();

    let pipeline_clone = pipeline.clone();
    ctrlc::set_handler(move || {
        // Send eos so that the encoder flushes
        pipeline_clone.send_event(gst::event::Eos::new());
    })
    .unwrap();

    let bus = pipeline.bus().unwrap();

    pipeline.set_state(gst::State::Playing).unwrap();

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        match msg.view() {
            MessageView::Eos(_) => {
                println!("EOS");
                break;
            }
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
