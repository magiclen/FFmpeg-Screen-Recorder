use std::process::Command;

use anyhow::{Context, anyhow};

/// Print the PulseAudio sources that `--audio-device` accepts.
pub fn list_devices() -> anyhow::Result<()> {
    let output = Command::new("pactl")
        .args(["list", "short", "sources"])
        .output()
        .context("cannot execute `pactl`; it is needed to list the audio devices")?;

    if !output.status.success() {
        return Err(anyhow!(
            "`pactl list short sources` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        ));
    }

    let text = String::from_utf8(output.stdout).context("`pactl` did not output UTF-8 text")?;

    println!("Pass one of these names to --audio-device:\n");

    for line in text.lines() {
        // Each line is `index<TAB>name<TAB>driver<TAB>sample spec<TAB>state`.
        if let Some(name) = line.split('\t').nth(1) {
            // A monitor source records what the speakers play, anything else records an input.
            let kind = if name.ends_with(".monitor") { "system audio" } else { "input" };

            println!("  {name}  ({kind})");
        }
    }

    println!(
        "\nUse --with-audio for the default system audio, --microphone for the default input, or \
         pass one of these names to --audio-device."
    );

    Ok(())
}
