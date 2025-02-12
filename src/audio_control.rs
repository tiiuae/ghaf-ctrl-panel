use futures::StreamExt;
use gio::ListStore;
use gtk::{gio, glib};
use std::ops::Deref;
use std::thread;
use tokio::runtime::Runtime;
use zbus::{Connection, Proxy};

use crate::audio_device_gobject::imp::AudioDeviceType;
use crate::audio_device_gobject::AudioDeviceGObject;
use crate::typed_list_store::imp::TypedListStore;

pub mod imp {
    use super::*;

    #[derive(Debug)]
    pub struct AudioControl {
        pub devices: TypedListStore<AudioDeviceGObject>,
        connection: Connection,
    }

    impl Default for AudioControl {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AudioControl {
        pub fn new() -> Self {
            let init_list = ListStore::new::<AudioDeviceGObject>(); //Self::fill_by_mock_data();
            let connection = Runtime::new()
                .unwrap()
                .block_on(Connection::session())
                .expect("AudioControl: Failed to connect to DBus session bus");

            Self {
                devices: init_list.into(),
                connection,
            }
        }

        fn fill_by_mock_data() -> ListStore {
            let init_store = ListStore::new::<AudioDeviceGObject>();
            let vec = vec![
                AudioDeviceGObject::new(
                    1,
                    AudioDeviceType::Sink as i32,
                    "Speakers".to_string(),
                    75,
                    false,
                ),
                AudioDeviceGObject::new(
                    2,
                    AudioDeviceType::Sink as i32,
                    "Headphones".to_string(),
                    50,
                    false,
                ),
                AudioDeviceGObject::new(
                    3,
                    AudioDeviceType::Source as i32,
                    "Microphone".to_string(),
                    100,
                    false,
                ),
                AudioDeviceGObject::new(
                    4,
                    AudioDeviceType::Source as i32,
                    "External Mic".to_string(),
                    85,
                    true,
                ),
            ];

            init_store.extend_from_slice(&vec);
            init_store
        }

        pub fn fetch_audio_devices(&self) {
            let connection = self.connection.clone();
            let (event_tx, event_rx) = async_channel::unbounded();

            thread::spawn(move || {
                let rt = Runtime::new().expect("AudioControl: Failed to create Tokio runtime");

                rt.block_on(async {
                    // Create a proxy to the org.ghaf.Audio interface
                    let proxy = Proxy::new(&connection, "org.ghaf.Audio", "/org/ghaf/Audio", "org.ghaf.Audio")
                        .await
                        .expect("AudioControl: Failed to create proxy");

                    // Stream to listen to the DeviceUpdated signal
                    let mut stream = match proxy.receive_signal("DeviceUpdated").await {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("AudioControl: Failed to receive signal: {}", e);
                            return;
                        }
                    };

                    // Subscribe to device updates
                    if let Err(e) = proxy.call_method("SubscribeToDeviceUpdatedSignal", &()).await {
                        eprintln!("AudioControl: Failed to subscribe to device updates: {}", e);
                        return;
                    }
                    println!("AudioControl: Subscribed to device updates.");

                    while let Some(signal) = stream.next().await {
                        match signal.body::<(i32, i32, String, i32, bool, bool, i32)>() {
                            Ok((id, device_type, name, volume, is_muted, is_default, event)) => {
                                println!(
                                    "AudioControl: DeviceUpdated - ID: {}, Type: {}, Name: {}, Volume: {}, Muted: {}, Default: {}, Event: {}",
                                    id, device_type, name, volume, is_muted, is_default, event
                                );

                                let _ = event_tx.send((id, device_type, name, volume, is_muted, event)).await;
                            }
                            Err(e) => {
                                eprintln!("AudioControl: Failed to parse signal: {}", e);
                            }
                        }
                    }
                });
            });

            let devices = self.devices.clone();
            //local future
            glib::spawn_future_local(async move {
                while let Ok((id, device_type, name, volume, is_muted, event)) =
                    event_rx.recv().await
                {
                    match event {
                        0 => {
                            // add
                            devices.append(&AudioDeviceGObject::new(
                                id,
                                device_type,
                                name.clone(),
                                volume,
                                is_muted,
                            ));
                            println!(
                                "AudioControl: Device added to the list - ID: {}, Type: {}, Name: {}, Volume: {}, Muted: {}",
                                id, device_type, name, volume, is_muted
                            );
                        }
                        1 => {
                            // update
                            if let Some(obj) = devices.iter().find(|obj| obj.id() == id) {
                                obj.update(device_type, name.clone(), volume, is_muted);
                                println!("AudioControl: Device with ID {id} has been updated");
                            }
                        }
                        2 => {
                            // remove
                            if let Some(pos) = devices.iter().position(|obj| obj.id() == id) {
                                devices.remove(pos as u32);
                                println!("AudioControl: Device with ID {id} has been deleted");
                            }
                        }
                        _ => {
                            println!("AudioControl: No such event");
                        }
                    }
                }
            });
        }

        pub fn get_devices_list(&self) -> ListStore {
            self.devices.deref().clone()
        }

        /*pub fn get_devices_list_by_type(&self, device_type: AudioDeviceType) -> ListStore {
            // Define the data structure for the ListStore: (i32, String, i32, bool, i32)
            let list_store = ListStore::new(&[
                i32::static_type(),
                String::static_type(),
                i32::static_type(),
                bool::static_type(),
            ]);

            // Clone and lock the devices
            let binding = self.devices.clone();
            let devices = binding.lock().unwrap();

            // Iterate, filter, and add rows to the ListStore
            devices
                .iter()
                .filter(|&(_, value)| value.0 == device_type as i32)
                .for_each(|(_, value)| {
                    list_store.insert_with_values(
                        None, // Append the row
                        &[(0, &value.0), (1, &value.1), (2, &value.2), (3, &value.3)],
                    );
                });

            list_store
        }*/

        pub fn set_device_volume(&self, id: i32, dev_type: i32, value: i32) {
            //same connection, new proxy
            let proxy = Runtime::new()
                .unwrap()
                .block_on(Proxy::new(
                    &self.connection,
                    "org.ghaf.Audio",
                    "/org/ghaf/Audio",
                    "org.ghaf.Audio",
                ))
                .expect("AudioControl(set_device_volume): Failed to create proxy");

            Runtime::new().unwrap().block_on(async {
                if let Err(e) = proxy
                    .call_method("SetDeviceVolume", &(id, dev_type, value))
                    .await
                {
                    eprintln!("AudioControl: Failed to set device volume: {}", e);
                }
            });
        }

        pub fn set_default_device(&self, id: i32) {
            //same connection, new proxy
            let proxy = Runtime::new()
                .unwrap()
                .block_on(Proxy::new(
                    &self.connection,
                    "org.ghaf.Audio",
                    "/org/ghaf/Audio",
                    "org.ghaf.Audio",
                ))
                .expect("AudioControl(set_default_device): Failed to create proxy");

            Runtime::new().unwrap().block_on(async {
                if let Err(e) = proxy.call_method("SetDefaultDevice", &(id)).await {
                    eprintln!("AudioControl: Failed to set default device: {}", e);
                }
            });
        }

        pub fn open_advanced_settings_widget(&self) {
            //same connection, new proxy
            let proxy = Runtime::new()
                .unwrap()
                .block_on(Proxy::new(
                    &self.connection,
                    "org.ghaf.Audio",
                    "/org/ghaf/Audio",
                    "org.ghaf.Audio",
                ))
                .expect("AudioControl(open_advanced_settings_widget): Failed to create proxy");

            Runtime::new().unwrap().block_on(async {
                if let Err(e) = proxy.call_method("Open", &()).await {
                    eprintln!(
                        "AudioControl: Failed to open advansed settings widget: {}",
                        e
                    );
                }
            });
        }

        pub fn unsubscribe_from_updates(&self) {
            let proxy = Runtime::new()
                .unwrap()
                .block_on(Proxy::new(
                    &self.connection,
                    "org.ghaf.Audio",
                    "/org/ghaf/Audio",
                    "org.ghaf.Audio",
                ))
                .expect("AudioControl(unsubscribe_from_updates): Failed to create proxy");

            Runtime::new().unwrap().block_on(async {
                if let Err(e) = proxy
                    .call_method("UnsubscribeFromDeviceUpdatedSignal", &())
                    .await
                {
                    eprintln!("AudioControl: Failed to unsubscribe from updates: {}", e);
                }
            });
        }
    }

    impl Drop for AudioControl {
        fn drop(&mut self) {
            println!("AudioControl: Unsubscribe from DeviceUpdatedSignal on dropping");
            self.unsubscribe_from_updates();
        }
    }
}
