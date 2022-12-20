#![allow(clippy::non_send_fields_in_send_ty)]

mod smartdevicemanagement;

use crate::smartdevicemanagement::api::StreamUrl::RtspUrl;
use std::sync::{Arc, Mutex};

use glib::prelude::*;
use glib::{translate::*, Value};
use gst::element_error;
use gst::prelude::*;
use gst::Element;
use gst::Pad;
use gst_rtsp::RTSPLowerTrans;
use gst_rtsp_sys::GstRTSPMessage;
use gst_sdp::SDPMessage;

use anyhow::{anyhow, Error};
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
#[display(fmt = "Received error from {}: {} (debug: {:?})", src, error, debug)]
struct ErrorMessage {
    src: String,
    error: String,
    debug: Option<String>,
    source: glib::Error,
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "ErrorValue")]
struct ErrorValue(Arc<Mutex<Option<Error>>>);

#[derive(Debug, Display, Error)]
#[display(fmt = "Could not get mount points")]
struct NoMountPoints;

fn get_test_rtsp_url() -> String {
    //return "rtsp://wowzaec2demo.streamlock.net/vod/mp4:BigBuckBunny_115k.mp4".to_owned();
    return "rtsps://stream-eu1-bravo.dropcam.com:443/sdm_live_stream/CiUA2vuxrxJvURetGs5Bj69Lac7JcuHaigVJkPxdi_je8X4EvOjKEnEAiN-WUy_b3qjTv7sG4wOtuoBnz8tNMGi1G7xVJNLo49gpYTrdWyddWR2pr3QwXy5cF8UEzQzMXP9XakITvqZN0KQLQmtAiFpi9YaFWUrxcZ6zG1vDo7gvx23LqkEdztVxbWlRFWGArBluHnwjhDADLQ?auth=g.0.eyJraWQiOiIyMzhiNTUxZmMyM2EyM2Y4M2E2ZTE3MmJjZTg0YmU3ZjgxMzAzMmM4IiwiYWxnIjoiUlMyNTYifQ.eyJpc3MiOiJuZXN0LXNlY3VyaXR5LWF1dGhwcm94eSIsInN1YiI6Im5lc3RfaWQ6bmVzdC1waG9lbml4LXByb2Q6MjEwODcxMCIsInBvbCI6IjNwLW9hdXRoLXNjb3BlLUFQSV9TRE1fU0VSVklDRS1jbGllbnQtMzQ2MzA2MTUxMjEyLWM1bWFnb3NyMWpzcWQyZDVwdTEzNWxqMjliazVsa3IxLmFwcHMuZ29vZ2xldXNlcmNvbnRlbnQuY29tIiwiZXhwIjoxNjcwNzAyNTk2fQ.oyX8kEqjxgVd3qt73q2Rqc99h8Aqxg939W9peYxXtgWxDCRzd_WrEaOLo1TKeoyYHiOS6_b010RFToP_9r41O7iOVhomoAdoGzqlQBYkt8HskzwiE7wP2vze_88qeYY1uJ-EaFoake60HKg2PC1piU6B9XxL4KdPpVwH6C1HnttG-AEgBpJ0cqyMpZu2YTz1GCVDbw5BAK7-RvTfXW2sl74GySUuX9fSqdo0Pa8ulU6hkiJzaA7xWOhJhLCjrDmTC-g4R-SB0bkOTnOaIAmePs9yPaYhrNKOmls5L64YS5mSJQ0t0G9ZSP56BSJ9MGmy4dAUyajx0u9UqVWoaqMqVw".to_string();
}

fn get_rtsp_url() -> String {
    let rtsp_camera_url = async {
        let api = smartdevicemanagement::client::SmartDeviceMgmtApi::new().await;
        let devices = api.device_list().await.unwrap();
        let first_rtsp = devices
            .cameras
            .iter()
            .find(|c| {
                c.details
                    .camera_live_stream
                    .supported_protocols
                    .contains(&"RTSP".to_string())
            })
            .expect("There should be at least one RTSP enabled camera");

        let rtsp_camera_id = &first_rtsp.name;

        let response = api.generate_rtsp_stream(&rtsp_camera_id).await?;

        anyhow::Ok(response.results.stream_urls)
    };

    match async_std::task::block_on(rtsp_camera_url) {
        Ok(RtspUrl(url)) => {
            println!("{:}", url);
            return url;
        }
        Err(e) => panic!("unexpected error {:?}", e),
    };
}

fn value_to_rtsp_message(value: &Value) -> Option<GstRTSPMessage> {
    unsafe {
        let ptr = value.as_ptr() as *mut GstRTSPMessage;
        ptr.as_ref().map(|m| m.to_glib_none().0)
    }
}

fn main_loop() -> Result<(), Error> {
    let pipeline = gst::Pipeline::default();

    let rtspsrc = gst::ElementFactory::make("rtspsrc")
        .property("location", get_rtsp_url())
        .property("latency", 2000 as u32)
        .property("protocols", RTSPLowerTrans::TCP)
        .property("do-rtcp", true)
        .property("is-live", true)
        .property("do-rtsp-keep-alive", true)
        .property("debug", true)
        .build()?;

    rtspsrc.connect("before-send", true, {
        move |input: &[Value]| unsafe {
            let message = input[1]
                .get::<GstRTSPMessage>()
                .expect("Can't get raw pointer")
                .cast::<GstRTSPMessage>();
            gst_rtsp_sys::gst_rtsp_message_dump(message);
            Some(true.to_value())
        }
    });

    let videoqueue = Arc::new(Mutex::new(gst::ElementFactory::make("queue").build()?));

    pipeline.add(&rtspsrc)?;

    let pipeline_weak = pipeline.downgrade();
    let videoqueue_clone = videoqueue.clone();

    rtspsrc.connect_pad_removed(move |_, src_pad| {
        let unlink = |videoqueue: &Element| -> Result<(), Error> {
            let sink_pad = videoqueue.static_pad("sink").unwrap();
            if sink_pad.is_linked() {
                return src_pad.unlink(&sink_pad).map_err(|e| e.into());
            } else {
                return Ok(());
            }
        };

        unlink(&videoqueue_clone.lock().unwrap()).expect("Error when unlinking");
    });

    let videoqueue_clone = videoqueue.clone();
    rtspsrc.connect_pad_added(move |rsrc, src_pad| {
        let pipeline = match pipeline_weak.upgrade() {
            Some(pipeline) => pipeline,
            None => return,
        };

        let insert_sink = |videoqueue: &Element| -> Result<(), Error> {
            let media_type = media_type_of(&src_pad)?;
            if !media_type.starts_with("video") {
                println!("ignoring pad with wrong media_type: {}", media_type);
                return Ok(());
            }

            let rtph264depay = gst::ElementFactory::make("rtph264depay").build()?;
            let h264parse = gst::ElementFactory::make("h264parse").build()?;
            let avdec_h264 = gst::ElementFactory::make("avdec_h264").build()?;
            let videosink = gst::ElementFactory::make("autovideosink")
                .property("sync", false)
                .build()?;

            let elements = &[
                &videoqueue,
                &rtph264depay,
                &h264parse,
                &avdec_h264,
                &videosink,
            ];
            pipeline.add_many(elements)?;
            gst::Element::link_many(elements)?;

            for e in elements {
                e.sync_state_with_parent()?
            }

            let sink_pad = videoqueue
                .static_pad("sink")
                .expect("video queue has no sinkpad");

            src_pad.link(&sink_pad)?;

            println!("Successfully linked rtspsrc to video chain");

            Ok(())
        };

        if let Err(err) = insert_sink(&videoqueue_clone.lock().unwrap()) {
            element_error!(
                rsrc,
                gst::LibraryError::Failed,
                ("Failed to insert sink"),
                details: gst::Structure::builder("error-details")
                                    .field("error", &ErrorValue(Arc::new(Mutex::new(Some(err)))))
                                    .build()
            );
        }
    });

    pipeline.set_state(gst::State::Playing)?;

    let bus = pipeline
        .bus()
        .expect("Pipeline without bus. Shouldn't happen!");

    for msg in bus.iter_timed(gst::ClockTime::NONE) {
        use gst::MessageView;

        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Error(err) => {
                pipeline.set_state(gst::State::Null)?;

                match err.details() {
                    Some(details) if details.name() == "error-details" => details
                        .get::<&ErrorValue>("error")
                        .unwrap()
                        .clone()
                        .0
                        .lock()
                        .unwrap()
                        .take()
                        .map(Result::Err)
                        .expect("error-details message without actual error"),
                    _ => Err(ErrorMessage {
                        src: msg
                            .src()
                            .map(|s| String::from(s.path_string()))
                            .unwrap_or_else(|| String::from("None")),
                        error: err.error().to_string(),
                        debug: err.debug(),
                        source: err.error(),
                    }
                    .into()),
                }?;
            }
            MessageView::StateChanged(s) => {
                println!(
                    "State changed from {:?}: {:?} -> {:?} ({:?})",
                    s.src().map(|s| s.path_string()),
                    s.old(),
                    s.current(),
                    s.pending()
                );
            }
            other => println!("MSG: {:#?}", other),
        }
    }

    pipeline.set_state(gst::State::Null)?;

    Ok(())
}

// Our custom media factory that creates a media input manually
mod media_factory {
    use super::*;

    use gst_rtsp_server::subclass::prelude::*;

    mod imp {
        use super::*;

        // This is the private data of our factory
        #[derive(Default)]
        pub struct Factory {}

        #[glib::object_subclass]
        impl ObjectSubclass for Factory {
            const NAME: &'static str = "RsRTSPMediaFactory";
            type Type = super::Factory;
            type ParentType = gst_rtsp_server::RTSPMediaFactory;
        }

        // Implementation of glib::Object virtual methods
        impl ObjectImpl for Factory {}

        // Implementation of gst_rtsp_server::RTSPMediaFactory virtual methods
        impl RTSPMediaFactoryImpl for Factory {
            fn create_element(&self, _url: &gst_rtsp::RTSPUrl) -> Option<gst::Element> {
                // Create a simple VP8 videotestsrc input
                let bin = gst::Bin::default();
                let src = gst::ElementFactory::make("videotestsrc")
                    // Configure the videotestsrc live
                    .property("is-live", true)
                    .build()
                    .unwrap();
                let enc = gst::ElementFactory::make("vp8enc")
                    // Produce encoded data as fast as possible
                    .property("deadline", 1i64)
                    .build()
                    .unwrap();

                // The names of the payloaders must be payX
                let pay = gst::ElementFactory::make("rtpvp8pay")
                    .name("pay0")
                    .build()
                    .unwrap();

                bin.add_many(&[&src, &enc, &pay]).unwrap();
                gst::Element::link_many(&[&src, &enc, &pay]).unwrap();

                Some(bin.upcast())
            }
        }
    }

    // This here defines the public interface of our factory and implements
    // the corresponding traits so that it behaves like any other RTSPMediaFactory
    glib::wrapper! {
        pub struct Factory(ObjectSubclass<imp::Factory>) @extends gst_rtsp_server::RTSPMediaFactory;
    }

    impl Default for Factory {
        fn default() -> Factory {
            glib::Object::new(&[])
        }
    }
}

fn run() -> Result<(), Error> {
    gst::init()?;
    main_loop()
}

fn main() {
    match run() {
        Ok(r) => r,
        Err(e) => eprintln!("Error! {}", e),
    }
}

fn media_type_of(pad: &Pad) -> anyhow::Result<String, anyhow::Error> {
    let caps = pad
        .current_caps()
        .expect("There were no caps for the new pad!");
    let structure = caps
        .structure(0)
        .expect("src pad doesn't have any structure");

    for n in 0..structure.n_fields() {
        let field_name = structure.nth_field_name(n).unwrap();
        if field_name.starts_with("media") {
            let media_type = structure.value(field_name).unwrap().get::<&str>().unwrap();
            return Ok(media_type.to_string());
        }
    }
    Err(anyhow!("No media field on pad"))
}
