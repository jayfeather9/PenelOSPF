mod config;
mod database;
mod interface;
mod interface_query;
mod interface_send;
mod lsa;
mod neighbor;
mod packets;
mod receiver;
mod route;
mod sender;
mod timer;

use pnet::datalink;
use tokio;
use tokio::sync::mpsc;

use crate::interface::Interface;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ints = datalink::interfaces();
    let mut my_ints: Vec<Interface> = Vec::new();
    let my_config = config::Config::new();
    let (db_mpsc_sdr, db_mpsc_rcvr) = mpsc::channel::<database::DatabaseRequest>(10);
    let (sdr_mpsc_sdr, sdr_mpsc_rcvr) = mpsc::channel::<sender::SenderRequest>(10);
    println!("=== Iterating over network interfaces ===");
    for i in ints {
        // Skip loopback interfaces
        if i.is_loopback() {
            continue;
        }
        my_ints.push(Interface::from(
            i,
            &my_config,
            db_mpsc_sdr.clone(),
            sdr_mpsc_sdr.clone(),
        ));
        println!(
            "Interface: {} {}",
            my_ints.last().unwrap().name,
            my_ints.last().unwrap().addr
        );
    }
    println!("=== Done iterating over network interfaces ===");

    let mut my_database =
        crate::database::LinkStateDatabase::from(my_config, db_mpsc_rcvr, my_ints.clone());

    let mut my_sender = crate::sender::OSPFPacketSender {
        request_channel: sdr_mpsc_rcvr,
    };

    tokio::spawn(async move {
        my_database.database_thread().await;
        println!("Database thread stopped");
    });
    tokio::spawn(async move {
        my_sender.sender_thread().await;
        println!("Sender thread stopped");
    });
    for i in my_ints {
        tokio::spawn(async move {
            i.clone().receiver().await;
            println!("Interface {} receiver thread stopped", i.name);
        });
    }
    // tokio::spawn(async move {
    //     my_ints[0].receiver().await;
    //     println!("Receiver thread stopped");
    // });

    // loop {
    //     my_ints[0].send_hello().await;
    //     tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    // }
    loop {}
}
