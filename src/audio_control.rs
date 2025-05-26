use futures::StreamExt;
use gio::ListStore;
use gtk::{gio, glib, prelude::*};
use log::{debug, error, warn};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;
use zbus::Connection;

use crate::audio_device_gobject::{imp::AudioDeviceType, AudioDeviceGObject};
use crate::typed_list_store::imp::TypedListStore;

mod imp {
    use zbus::proxy;

    #[proxy(
        interface = "org.ghaf.Audio",
        default_service = "org.ghaf.Audio",
        default_path = "/org/ghaf/Audio"
    )]
    pub trait GhafAudio {
        async fn subscribe_to_device_updated_signal(&self) -> zbus::Result<()>;
        async fn unsubscribe_from_device_updated_signal(&self) -> zbus::Result<()>;
        async fn set_device_volume(&self, id: i32, dev_type: i32, value: i32) -> zbus::Result<i32>;
        async fn set_device_mute(&self, id: i32, dev_type: i32, mute: bool) -> zbus::Result<i32>;
        async fn make_device_default(&self, id: i32, dev_type: i32) -> zbus::Result<i32>;
        async fn open(&self) -> zbus::Result<()>;

        #[zbus(signal)]
        fn device_updated(
            &self,
            id: i32,
            device_type: i32,
            name: String,
            volume: i32,
            is_muted: bool,
            is_default: bool,
            event: i32,
        ) -> zbus::Result<()>;
    }
}

type Proxy = imp::GhafAudioProxy<'static>;

#[derive(Debug, Default, Clone)]
enum ConnectionState {
    #[default]
    Disconnected,
    Connecting(async_channel::Receiver<()>),
    Connected(Proxy),
}

impl ConnectionState {
    fn proxy(&self) -> Option<Proxy> {
        match self {
            ConnectionState::Connected(p) => Some(p.clone()),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct AudioControl {
    devices: TypedListStore<AudioDeviceGObject>,
    connection: Rc<RefCell<ConnectionState>>,
}

impl Default for AudioControl {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioControl {
    pub fn new() -> Self {
        let devices = TypedListStore::new();
        let (tx, rx) = async_channel::bounded(1);
        let connection = Rc::new(RefCell::new(ConnectionState::Connecting(rx)));
        #[cfg(not(feature = "mock"))]
        glib::spawn_future_local(glib::clone!(
            #[strong]
            connection,
            async move {
                let _tx = tx;
                let bus = Connection::session().await;
                if let Ok(bus) = bus {
                    let proxy = imp::GhafAudioProxy::new(&bus).await;
                    *connection.borrow_mut() = if let Ok(proxy) = proxy {
                        ConnectionState::Connected(proxy)
                    } else {
                        ConnectionState::Disconnected
                    };
                } else {
                    *connection.borrow_mut() = ConnectionState::Disconnected;
                }
            }
        ));
        #[cfg(feature = "mock")]
        glib::spawn_future_local(glib::clone!(
            #[strong]
            devices,
            async move {
                Self::fill_by_mock_data(&devices);
            }
        ));
        Self {
            devices,
            connection,
        }
    }

    #[cfg(feature = "mock")]
    fn fill_by_mock_data(list: &TypedListStore<AudioDeviceGObject>) {
        list.append(&AudioDeviceGObject::new(
            1,
            AudioDeviceType::Sink as i32,
            "Speakers".to_string(),
            75,
            false,
            false,
        ));
        list.append(&AudioDeviceGObject::new(
            2,
            AudioDeviceType::Sink as i32,
            "Headphones".to_string(),
            50,
            true,
            true,
        ));
        list.append(&AudioDeviceGObject::new(
            3,
            AudioDeviceType::Source as i32,
            "Microphone".to_string(),
            100,
            false,
            true,
        ));
        list.append(&AudioDeviceGObject::new(
            4,
            AudioDeviceType::Source as i32,
            "External Mic".to_string(),
            85,
            true,
            false,
        ));
    }

    pub fn fetch_audio_devices(&self, cb: impl Fn(TypedListStore<AudioDeviceGObject>) + 'static) {
        let devices = self.devices.clone();

        #[cfg(not(feature = "mock"))]
        self.with_proxy(async move |proxy| {
            // Stream to listen to the DeviceUpdated signal
            let mut stream = match proxy.receive_device_updated().await {
                Ok(s) => s,
                Err(e) => {
                    error!("AudioControl: Failed to receive signal: {e}");
                    return;
                }
            };

            // Subscribe to device updates
            if let Err(e) = proxy.subscribe_to_device_updated_signal().await {
                error!("AudioControl: Failed to subscribe to device updates: {e}");
                return;
            }
            debug!("AudioControl: Subscribed to device updates.");

            glib::spawn_future_local(glib::clone!(
                    #[strong]
                    devices,
                    async move {
                        // TODO: Currently cannot tell when initial device dump is finished
                        glib::timeout_future_seconds(1).await;
                        cb(devices);
                    }
            ));

            while let Some(signal) = stream.next().await {
                match signal.args() {
                    Ok(imp::DeviceUpdatedArgs {
                        id,
                        device_type,
                        name,
                        volume,
                        is_muted,
                        is_default,
                        event,
                        ..
                    }) => {
                        debug!(
                            "AudioControl: DeviceUpdated - ID: {id}, Type: {device_type}, Name: {name}, \
                                    Volume: {volume}, Muted: {is_muted}, Default: {is_default}, Event: {event}"
                        );

                        if name.to_lowercase().contains("monitor") {
                            debug!("AudioControl: Monitor ignored {id}, {device_type}, {name}!");
                        } else {
                            match event {
                                0 => {
                                    // add
                                    debug!(
                                        "AudioControl: Device added to the list - ID: {id}, Type: {device_type}, \
                                             Name: {name}, Volume: {volume}, Muted: {is_muted}"
                                    );
                                    devices.append(&AudioDeviceGObject::new(
                                        id,
                                        device_type,
                                        name,
                                        volume,
                                        is_muted,
                                        is_default,
                                    ));
                                }
                                1 => {
                                    // update
                                    if let Some(obj) = devices
                                        .iter()
                                        .find(|obj| {
                                            //id and type as composite key
                                            (obj.id() == id) && (obj.dev_type() == device_type)
                                        })
                                    {
                                        obj.update(device_type, name, volume, is_muted, is_default);
                                        debug!(
                                            "AudioControl: Device with ID {id} has been updated"
                                        );
                                    } else {
                                        warn!("AudioControl: could not find device {id}");
                                    }
                                }
                                2 => {
                                    // remove
                                    devices.retain(|o| o.downcast_ref::<AudioDeviceGObject>().is_none_or(|o| o.id() != id || o.dev_type() != device_type));
                                }
                                _ => {
                                    debug!("AudioControl: No such event");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("AudioControl: Failed to parse signal: {e}");
                    }
                }
            }
        });

        #[cfg(feature = "mock")]
        glib::spawn_future_local(glib::clone!(
            #[strong]
            devices,
            async move {
                glib::timeout_future_seconds(1).await;
                cb(devices);
            }
        ));
    }

    pub fn get_devices_list(&self) -> ListStore {
        self.devices.deref().clone()
    }

    pub fn set_device_volume(&self, id: i32, dev_type: i32, value: i32) {
        self.with_proxy(async move |proxy| {
            if let Err(e) = proxy.set_device_volume(id, dev_type, value).await {
                error!("AudioControl: Failed to set device volume: {e}");
            }
        });
    }

    pub fn set_device_mute(&self, id: i32, dev_type: i32, mute: bool) {
        self.with_proxy(async move |proxy| {
            if let Err(e) = proxy.set_device_mute(id, dev_type, mute).await {
                error!("AudioControl: Failed to set device volume: {e}");
            }
        });
    }

    pub fn set_default_device(&self, id: i32, dev_type: i32) {
        self.with_proxy(async move |proxy| {
            if let Err(e) = proxy.make_device_default(id, dev_type).await {
                error!("AudioControl: Failed to set default device: {e}");
            }
        });
    }

    pub fn open_advanced_settings_widget(&self) {
        self.with_proxy(async move |proxy| {
            if let Err(e) = proxy.open().await {
                error!("AudioControl: Failed to open advansed settings widget: {e}");
            }
        });
    }

    fn unsubscribe_from_updates(&self) {
        self.with_proxy(async move |proxy| {
            if let Err(e) = proxy.unsubscribe_from_device_updated_signal().await {
                error!("AudioControl: Failed to unsubscribe from updates: {e}");
            }
        });
    }

    fn proxy(&self) -> impl std::future::Future<Output = Option<Proxy>> {
        let conn = self.connection.clone();

        async move {
            let rx = match &*conn.borrow() {
                ConnectionState::Disconnected => return None,
                ConnectionState::Connected(p) => return Some(p.clone()),
                ConnectionState::Connecting(rx) => rx.clone(),
            };

            let _ = rx.recv().await;
            conn.borrow().proxy()
        }
    }

    fn with_proxy(&self, f: impl AsyncFnOnce(imp::GhafAudioProxy) + 'static) {
        let proxy = self.proxy();
        glib::spawn_future_local(async move {
            if let Some(proxy) = proxy.await {
                f(proxy).await;
            }
        });
    }
}

impl Drop for AudioControl {
    fn drop(&mut self) {
        debug!("AudioControl: Unsubscribe from DeviceUpdatedSignal on dropping");
        self.unsubscribe_from_updates();
    }
}
