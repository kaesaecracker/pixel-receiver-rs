#![deny(clippy::all)]

use std::io::ErrorKind;
use std::net::UdpSocket;
use std::sync::{mpsc, RwLock};
use std::time::Duration;

use clap::Parser;
use log::{info, warn};
use servicepoint2::{
    ByteGrid, Command, PixelGrid, PIXEL_HEIGHT, PIXEL_WIDTH, TILE_HEIGHT,
    TILE_WIDTH,
};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::execute_command::execute_command;
use crate::font::BitmapFont;
use crate::gui::{App, AppEvents};

mod execute_command;
mod font;
mod gui;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(long, default_value = "0.0.0.0:2342")]
    bind: String,
    #[arg(short, long, default_value_t = false)]
    spacers: bool,
    #[arg(short, long, default_value_t = false)]
    red: bool,
    #[arg(short, long, default_value_t = false)]
    green: bool,
    #[arg(short, long, default_value_t = false)]
    blue: bool,
}

fn main() {
    env_logger::init();

    let mut cli = Cli::parse();
    if !(cli.red || cli.blue || cli.green) {
        cli.green = true;
    }

    info!("starting with args: {:?}", &cli);
    let socket = UdpSocket::bind(&cli.bind).expect("could not bind socket");
    socket
        .set_nonblocking(true)
        .expect("could not enter non blocking mode");

    let font = BitmapFont::load_file("Web437_IBM_BIOS.woff");

    let display = RwLock::new(PixelGrid::new(
        PIXEL_WIDTH as usize,
        PIXEL_HEIGHT as usize,
    ));

    let mut luma = ByteGrid::new(TILE_WIDTH as usize, TILE_HEIGHT as usize);
    luma.fill(u8::MAX);
    let luma = RwLock::new(luma);

    run(&display, &luma, socket, font, &cli);
}

fn run(
    display_ref: &RwLock<PixelGrid>,
    luma_ref: &RwLock<ByteGrid>,
    socket: UdpSocket,
    font: BitmapFont,
    cli: &Cli,
) {
    let (stop_udp_tx, stop_udp_rx) = mpsc::channel();

    let mut app = App::new(display_ref, luma_ref, stop_udp_tx, cli);

    let event_loop = EventLoop::with_user_event()
        .build()
        .expect("could not create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let event_proxy = event_loop.create_proxy();

    std::thread::scope(move |scope| {
        let udp_thread = scope.spawn(move || {
            let mut buf = [0; 8985];

            while stop_udp_rx.try_recv().is_err() {
                let (amount, _) = match socket.recv_from(&mut buf) {
                    Err(err) if err.kind() == ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(1));
                        continue;
                    }
                    Ok(result) => result,
                    other => other.unwrap(),
                };

                if amount == buf.len() {
                    warn!(
                        "the received package may have been truncated to a length of {}",
                        amount
                    );
                }

                let vec = buf[..amount].to_vec();
                let package = servicepoint2::Packet::from(vec);

                let command = match Command::try_from(package) {
                    Err(err) => {
                        warn!("could not read command for packet: {:?}", err);
                        continue;
                    }
                    Ok(val) => val,
                };

                if !execute_command(command, &font, display_ref, luma_ref) {
                    // hard reset
                    event_proxy
                        .send_event(AppEvents::UdpThreadClosed)
                        .expect("could not send close event");
                    break;
                }

                event_proxy
                    .send_event(AppEvents::UdpPacketHandled)
                    .expect("could not send packet handled event");
            }
        });

        event_loop
            .run_app(&mut app)
            .expect("could not run event loop");

        udp_thread.join().expect("could not join udp thread");
    });
}
