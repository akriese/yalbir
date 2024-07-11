use bleps::ad_structure::{
    create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
};
use bleps::async_attribute_server::AttributeServer;
use bleps::asynch::Ble;
use bleps::attribute_server::NotificationData;
use bleps::gatt;
use core::cell::RefCell;
use esp_hal::gpio::Gpio14;
use esp_hal::gpio::Input;
use esp_wifi::ble::controller::asynch::BleConnector;

use crate::util::commands::handle_wireless_input;

#[embassy_executor::task]
pub(crate) async fn ble_handling(
    mut ble: Ble<BleConnector<'static>>,
    pin: RefCell<Input<'static, Gpio14>>,
) {
    loop {
        log::info!("{:?}", ble.init().await);
        log::info!("{:?}", ble.cmd_set_le_advertising_parameters().await);
        log::info!(
            "{:?}",
            ble.cmd_set_le_advertising_data(
                create_advertising_data(&[
                    AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                    AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
                    AdStructure::CompleteLocalName("antons-esp"),
                ])
                .unwrap()
            )
            .await
        );
        log::info!("{:?}", ble.cmd_set_le_advertise_enable(true).await);

        log::info!("started advertising");

        let mut write_callback = |offset: usize, data: &[u8]| {
            log::info!("RECEIVED: Offset {}, data {:?}", offset, data);
            handle_wireless_input(core::str::from_utf8(data).unwrap());
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

        let counter = RefCell::new(0u8);
        let counter = &counter;

        let mut notifier = || async {
            pin.borrow_mut().wait_for_rising_edge().await;
            log::info!("button pressed");
            let mut data = [0u8; 13];
            data.copy_from_slice(b"Notification0");
            {
                let mut counter = counter.borrow_mut();
                data[data.len() - 1] += *counter;
                *counter = (*counter + 1) % 10;
            }
            NotificationData::new(socket_handle, &data)
        };

        match srv.run(&mut notifier).await {
            Ok(()) => (),
            Err(msg) => log::info!("{:?}", msg),
        }
    }
}
