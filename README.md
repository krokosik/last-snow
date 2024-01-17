# Last Snow Input+Display

This code should be run on a prepared Raspberry Pi connected to the touch screen display. In case the provided SD card cannot be used, an OS image can be downloaded from [here](https://drive.google.com/file/d/1C69SJAPZ8itXk2L6r_15shJzHu12Gr_M/view?usp=sharing).

## Setup

### Hardware

- connect the **official** power supply to the Raspberry Pi USB-C port
- connect the keyboard to one of the USB ports
- wait for the Raspberry Pi to boot, when you see the desktop, don't click anything, the program will start automatically after a few seconds

### Network

While the program does not require internet connection to work, it needs to be on the same network as the host PC. There are 3 ways to do this:

1. Connect the Raspberry Pi to the network via Ethernet cable. In case only one cable is available, a switch can be used to connect both the PC and the Raspberry Pi to the same cable.

2. Another way is to use a cross-over Ethernet cable to connect the PC and the Raspberry Pi directly. This method is not recommended, as it requires a special cable.

3. If all else fails you can connect the Raspberry Pi to Wifi. To do this, you need to first kill the main program, in order to see the taskbar. For this, see how to connect via SSH below. Once you have the taskbar, click on the network icon and connect to the network.

Make sure the Pi is connected by pinging it from the PC. Open the terminal and execute `ping last-snow.local`.

### Software

#### CSV files

Each sentence submitted by the user is saved in a CSV file. Once a certain number of sentences is accumulated, the CSV file will be placed in the `sentences` network folder. You can access it directly in Windows or TouchDesigner by going to `\\last-snow.local\sentences`. You will be asked for a username and password, use `last_snow` and `La$t$n0w` respectively.

#### OSC

It is possible to send OSC messages to the Raspberry Pi to control the program. The OSC port is `7000`, so the messages need to be sent to `last-snow.local:7000`. The following messages are supported:

- `/max_characters` - sets the maximum number of characters allowed in a sentence. The default value is `160`.
- `/max_sentences_per_csv` - sets the maximum number of sentences per CSV file. The default value is `100`. Keep in mind that changing this will not affect the existing CSV files.
- `/td_osc_address` - sets the OSC address (`ip:port`) to which the program will send OSC ping messages upon each new sentence. This can be then used to trigger events in TouchDesigner. Keep in mind there is no default value. You can use the `Local Address` from the `OSC In` DAT, but keep in mind to pick an address from the correct network interface. The program sends an OSC message to `/new_row` address, followed by the submitted sentence.
- `/remove_output_csv` - removes the output CSV file. Use its filename as argument, i.e. `/remove_output_csv 0.csv`.
- `/remove_tmp_csv` - removes the temporary CSV file.
- `/remove_all_csv` - removes all CSV files.

All of configuration values are saved in `~/.config/last-snow/.settings` and will be loaded on startup, so changes are persistent between Raspberry Pi reboots.

#### Japanese Keyboard

In case there is no Japanese keyboard available, there are shortcuts for toggling between different Japanese input methods:

- `Ctrl + J` toggles between Hiragana and Romaji
- `Ctrl + Shift + .` toggles between Hiragana and Katakana
- `Space` is used to cycle between different suggestions

All system keyboard shortcuts like `Alt+F4` or `Ctrl+Alt+Del` are disabled. For submitting text, apart from the touch button, you can also use `Shift+Enter` on the keyboard.

#### SSH

In case something goes wrong and you need to access the Raspberry Pi directly, you can do so via SSH. Open your terminal and execute: `ssh last_snow@last-snow.local` and type in the same password as for the shared network folder. You now have access to the Raspberry Pi command line. To kill the main program, execute `lsk`. To update it, execute `lsu`. To restart the Raspberry Pi, execute `sudo reboot`. The CSV files are located inside `~/Public`. There are also logs in `~/.config/last-snow/logs/last-snow.log`, which you can read using `cat ~/.config/last-snow/logs/last-snow.log`. Alternatively you can run `lsk` and run the program from SSH using `DISPLAY=:0 last-snow`.

#### Updating

In case something needs updating, let me know in detail what the issue is and I will update the package version. It will be then automatically built in the cloud and you need to update with `lsu`. You can do so via SSH, see above.