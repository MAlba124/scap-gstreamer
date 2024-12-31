// Copyright (C) 2024 Marcus L. Hanestad <marlhan@proton.me>

use std::sync::LazyLock;
use std::sync::Mutex;

use gst::glib;
use gst::prelude::*;

use gst_base::prelude::BaseSrcExt;
use gst_base::subclass::base_src::CreateSuccess;
use gst_base::subclass::prelude::*;
use scap::capturer::Capturer;

const DEFAULT_SHOW_CURSOR: bool = true;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "scapsrc",
        gst::DebugColorFlags::empty(),
        Some("Scap screencast source"),
    )
});

#[derive(Default)]
struct Settings {
    pub show_cursor: bool,
}

#[derive(Default)]
struct State {
    info: Option<gst_video::VideoInfo>,
    width: i32,
    height: i32,
}

pub struct ScapSrc {
    settings: Mutex<Settings>,
    capturer: Mutex<Option<Capturer>>,
    state: Mutex<State>,
}

impl Default for ScapSrc {
    fn default() -> Self {
        Self {
            settings: Mutex::new(Default::default()),
            capturer: Mutex::new(None),
            state: Mutex::new(Default::default()),
        }
    }
}

impl ScapSrc {}

#[glib::object_subclass]
impl ObjectSubclass for ScapSrc {
    const NAME: &'static str = "ScapSrc";
    type Type = super::ScapSrc;
    type ParentType = gst_base::PushSrc;
}

impl ObjectImpl for ScapSrc {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecBoolean::builder("show-cursor")
                    .nick("Show cursor")
                    .blurb("Wheter to capture the cursor or not")
                    .default_value(DEFAULT_SHOW_CURSOR)
                    .mutable_ready()
                    .build(),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn constructed(&self) {
        self.parent_constructed();
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "show-cursor" => {
                let mut settings = self.settings.lock().unwrap();
                // TODO: Log
                settings.show_cursor = value.get().expect("type checked upstream");
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "show-cursor" => {
                let settings = self.settings.lock().unwrap();
                settings.show_cursor.to_value()
            }
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for ScapSrc {}

impl ElementImpl for ScapSrc {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "Scap screencast source",
                "Source/Video",
                "Scap screencast source",
                "Marcus L. Hanestad <marlhan@proton.me>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let caps = gst_video::VideoCapsBuilder::new()
                .format_list([
                    gst_video::VideoFormat::Rgb,
                    gst_video::VideoFormat::Rgbx,
                    gst_video::VideoFormat::Xbgr,
                    gst_video::VideoFormat::Bgrx,
                    gst_video::VideoFormat::Bgrx,
                    gst_video::VideoFormat::Bgra,
                ])
                .build();
            let src_pad_template = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            vec![src_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }

    fn change_state(
        &self,
        transition: gst::StateChange,
    ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
        self.parent_change_state(transition)
    }
}

impl BaseSrcImpl for ScapSrc {
    fn start(&self) -> Result<(), gst::ErrorMessage> {
        let mut capturer = self.capturer.lock().unwrap();
        let settings = self.settings.lock().unwrap();

        if let Some(mut capturer) = capturer.take() {
            capturer.stop_capture();
        }

        let mut new_capturer = Capturer::build(scap::capturer::Options {
            fps: 25,
            show_cursor: settings.show_cursor,
            show_highlight: true,
            target: None,
            crop_area: None,
            output_type: scap::frame::FrameType::BGR0,
            output_resolution: scap::capturer::Resolution::Captured,
            excluded_targets: None,
        })
        .map_err(|e| gst::error_msg!(gst::LibraryError::Init, ["{e}"]))?;

        new_capturer.start_capture();

        *capturer = Some(new_capturer);

        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        match self.capturer.lock().unwrap().take() {
            Some(mut c) => c.stop_capture(),
            None => {
                return Err(gst::error_msg!(gst::LibraryError::Shutdown, [
                    "Missing capturer"
                ]));
            }
        }

        Ok(())
    }

    fn set_caps(&self, caps: &gst::Caps) -> Result<(), gst::LoggableError> {
        let info = gst_video::VideoInfo::from_caps(caps).map_err(|_| {
            gst::loggable_error!(CAT, "Failed to build `VideoInfo` from caps {}", caps)
        })?;

        gst::debug!(CAT, imp = self, "Configuring for caps {}", caps);

        let mut state = self.state.lock().unwrap();

        let (new_width, new_height) = (info.width(), info.height());

        self.obj().set_blocksize(4 * new_width * new_height);

        state.info = Some(info);
        state.width = new_width as i32;
        state.height = new_height as i32;

        Ok(())
    }

    fn fixate(&self, /*mut*/ caps: gst::Caps) -> gst::Caps {
        self.parent_fixate(caps)
    }

    fn is_seekable(&self) -> bool {
        false
    }
}

struct FrameInfo {
    width: u32,
    height: u32,
    gst_v_format: gst_video::VideoFormat,
}

impl FrameInfo {
    pub fn new(frame: &scap::frame::Frame) -> Option<Self> {
        Some(match frame {
            scap::frame::Frame::RGB(f) => Self {
                width: f.width as u32,
                height: f.height as u32,
                gst_v_format: gst_video::VideoFormat::Rgb,
            },
            scap::frame::Frame::RGBx(f) => Self {
                width: f.width as u32,
                height: f.height as u32,
                gst_v_format: gst_video::VideoFormat::Rgbx,
            },
            scap::frame::Frame::XBGR(f) => Self {
                width: f.width as u32,
                height: f.height as u32,
                gst_v_format: gst_video::VideoFormat::Xbgr,
            },
            scap::frame::Frame::BGRx(f) => Self {
                width: f.width as u32,
                height: f.height as u32,
                gst_v_format: gst_video::VideoFormat::Bgrx,
            },
            scap::frame::Frame::BGR0(f) => Self {
                width: f.width as u32,
                height: f.height as u32,
                gst_v_format: gst_video::VideoFormat::Bgrx,
            },
            scap::frame::Frame::BGRA(f) => Self {
                width: f.width as u32,
                height: f.height as u32,
                gst_v_format: gst_video::VideoFormat::Bgra,
            },
            _ => return None,
        })
    }
}

impl PushSrcImpl for ScapSrc {
    fn create(&self, _: Option<&mut gst::BufferRef>) -> Result<CreateSuccess, gst::FlowError> {
        let Some(ref cap) = *self.capturer.lock().unwrap() else {
            return Err(gst::FlowError::NotNegotiated);
        };

        let frame = cap.get_next_frame().map_err(|e| {
            gst::error!(CAT, imp = self, "Failed to get next frame: {e}");
            gst::FlowError::Error
        })?;

        let state = self.state.lock().unwrap();

        let neg_format = match &state.info {
            Some(i) => i.format(),
            None => return Err(gst::FlowError::NotNegotiated),
        };

        let Some(frame_info) = FrameInfo::new(&frame) else {
            gst::error!(CAT, imp = self, "Unsupported frame format received");
            return Err(gst::FlowError::Error);
        };

        if (state.width, state.height) != (frame_info.width as i32, frame_info.height as i32)
            || neg_format != frame_info.gst_v_format
        {
            gst::debug!(
                CAT,
                imp = self,
                "Resolutions differ. Will try to renegotiate"
            );

            let new_video_info = gst_video::VideoInfo::builder(
                frame_info.gst_v_format,
                frame_info.width,
                frame_info.height,
            )
            .build()
            .map_err(|e| {
                gst::error!(CAT, imp = self, "Failed to create vidoe info: {e}");
                gst::FlowError::Error
            })?;

            let new_caps = new_video_info.to_caps().map_err(|e| {
                gst::error!(CAT, imp = self, "Failed to create caps: {e}");
                gst::FlowError::Error
            })?;

            drop(state);

            if let Err(e) = self.obj().set_caps(&new_caps) {
                gst::error!(CAT, imp = self, "Failed to set caps: {e}");
                return Err(gst::FlowError::Error);
            }
        }

        match frame {
            scap::frame::Frame::RGB(f) => {
                Ok(CreateSuccess::NewBuffer(gst::Buffer::from_slice(f.data)))
            }
            scap::frame::Frame::RGBx(f) => {
                Ok(CreateSuccess::NewBuffer(gst::Buffer::from_slice(f.data)))
            }
            scap::frame::Frame::XBGR(f) => {
                Ok(CreateSuccess::NewBuffer(gst::Buffer::from_slice(f.data)))
            }
            scap::frame::Frame::BGRx(f) => {
                Ok(CreateSuccess::NewBuffer(gst::Buffer::from_slice(f.data)))
            }
            scap::frame::Frame::BGR0(f) => {
                Ok(CreateSuccess::NewBuffer(gst::Buffer::from_slice(f.data)))
            }
            scap::frame::Frame::BGRA(f) => {
                Ok(CreateSuccess::NewBuffer(gst::Buffer::from_slice(f.data)))
            }
            _ => unreachable!(), // Yuv format should already have returned an error
        }
    }
}
