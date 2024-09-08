# x11-idle-sync

x11-idle-sync is a lightweight utility that synchronize X11 screensaver idle time with the logind's idle hint.

If you use bare X11 without a desktop environment (KDE, GNOME, etc.), you may find `/etc/systemd/logind.conf` not working as expected to suspend or hibernate your system. This is probably because logind's idle hint is unmanaged. **x11-idle-sync** is a workaround to this problem by monitoring X11 user activity and setting the idle hint accordingly.

Alternatively, you may want to use `xss-lock` to manage idle hint for you if you use `xset s` to set screensaver timeout.

## Check for idle hint

Try run the following command:

```
while true; loginctl -p IdleHint -p IdleSinceHint show-session; sleep 1; done
```

If you ever see these values changing when the system is idle, it means the idle hint is being set properly. If not, you may need this utility.

## Usage

Run x11-idle-sync with the following options:

```
x11-idle-sync [OPTIONS]
```

Options:
- `-t, --idle-threshold <SECONDS>`: Set the idle threshold in seconds (default: 300)
- `-N, --no-reset-on-exit`: Disable resetting idle hint to false on exit
- `-1, --one-shot`: Run as a one-shot idle check (check once and exit)

Examples:

1. Run with default settings:
   ```
   x11-idle-sync
   ```

2. Set a custom idle threshold of 10 minutes:
   ```
   x11-idle-sync --idle-threshold 600
   ```

3. Perform a one-shot idle check and set IdleHint accordingly:
   ```
   x11-idle-sync --one-shot
   ```

4. Run continuously without resetting idle hint on exit:
   ```
   x11-idle-sync --no-reset-on-exit
   ```

## Notes

- Requires X11 and systemd login manager.
- Must be run in a user session with access to the X11 display and D-Bus.
