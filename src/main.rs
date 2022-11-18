#![allow(clippy::non_send_fields_in_send_ty)]

mod smartdevicemanagement;

use gst_rtsp_server::prelude::*;

use anyhow::Error;
use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
#[display(fmt = "Could not get mount points")]
struct NoMountPoints;

fn main_loop() -> Result<(), Error> {
    let main_loop = glib::MainLoop::new(None, false);
    let server = gst_rtsp_server::RTSPServer::new();

    let mounts = gst_rtsp_server::RTSPMountPoints::new();
    server.set_mount_points(Some(&mounts));

    let mounts = server.mount_points().ok_or(NoMountPoints)?;

    let factory = media_factory::Factory::default();
    factory.set_shared(true);

    mounts.add_factory("/test", &factory);

    let id = server.attach(None)?;

    println!(
        "Stream ready at rtsp://127.0.0.1:{}/test",
        server.bound_port()
    );

    main_loop.run();

    id.remove();

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
