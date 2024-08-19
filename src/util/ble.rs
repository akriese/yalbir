use alloc::string::{String, ToString};
use bleps::ad_structure::{
    create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
};
use bleps::async_attribute_server::AttributeServer;
use bleps::asynch::Ble;
use bleps::attribute_server::NotificationData;
use bleps::gatt;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::Timer;
use esp_wifi::ble::controller::asynch::BleConnector;

use crate::util::commands::handle_wireless_input;

const INFO_PACKET_LENGTH: usize = (bleps::attribute_server::MTU - 3) as usize;

static COMMAND_REPLY: Signal<CriticalSectionRawMutex, ()> = Signal::new();
static mut REPLY: String = String::new();

#[embassy_executor::task]
pub(crate) async fn ble_handling(mut ble: Ble<BleConnector<'static>>) {
    loop {
        log::info!("{:?}", ble.init().await);
        log::info!("{:?}", ble.cmd_set_le_advertising_parameters().await);
        log::info!(
            "{:?}",
            ble.cmd_set_le_advertising_data(
                create_advertising_data(&[
                    AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                    AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
                    AdStructure::CompleteLocalName("carls house"),
                ])
                .unwrap()
            )
            .await
        );
        log::info!("{:?}", ble.cmd_set_le_advertise_enable(true).await);

        log::info!("started advertising");

        let mut write_callback = |offset: usize, data: &[u8]| {
            log::info!("RECEIVED: Offset {}, data {:?}", offset, data);
            let res = handle_wireless_input(core::str::from_utf8(data).unwrap());
            if let Err(err) = res {
                unsafe { REPLY = err.to_string() };
                COMMAND_REPLY.signal(())
            }
        };

        gatt!([service {
            uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
            characteristics: [characteristic {
                name: "socket",
                uuid: "987312e0-2354-11eb-9f10-fbc30a62cf38",
                notify: true,
                write: write_callback,
            },],
        },]);

        let mut rng = bleps::no_rng::NoRng;
        let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes, &mut rng);

        let mut notifier = || async {
            if unsafe { REPLY.is_empty() } {
                COMMAND_REPLY.wait().await;
            }

            // this is somehow needed to not cause weird behavior on the client side
            // as the messages seem to be sent too quickly
            Timer::after_millis(50).await;

            unsafe {
                let next_notification: String;
                if REPLY.len() < INFO_PACKET_LENGTH {
                    next_notification = REPLY.to_string();
                    REPLY = "".to_string();
                } else {
                    let (head, tail) = REPLY.split_at(INFO_PACKET_LENGTH);
                    next_notification = head.to_string();
                    REPLY = tail.to_string();
                }

                NotificationData::new(socket_handle, next_notification.as_bytes())
            }
        };

        match srv.run(&mut notifier).await {
            Ok(()) => (),
            Err(msg) => log::info!("{:?}", msg),
        }
    }
}
