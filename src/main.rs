use vigem_client::{Client, TargetId, XButtons, XGamepad, Xbox360Wired};

fn main() {
    let client = match Client::connect() {
        Ok(c) => {
            println!("Connected to ViGEm bus!");
            c
        }
        Err(e) => {
            eprintln!("Failed to connect to ViGEm bus: {e}");
            eprintln!("Make sure ViGEmBus driver is installed.");
            return;
        }
    };

    let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);

    target.plugin().expect("Failed to plug in controller");
    target.wait_ready().expect("Controller not ready");
    println!("Virtual controller connected! Check joy.cpl now.");

    let mut gamepad = XGamepad {
        buttons: XButtons { raw: XButtons::A },
        ..Default::default()
    };
    target.update(&gamepad).expect("Failed to send input");
    println!("A button pressed!");

    std::thread::sleep(std::time::Duration::from_secs(5));

    gamepad.buttons = XButtons { raw: 0 };
    target.update(&gamepad).expect("Failed to release input");
    println!("Done!");
}