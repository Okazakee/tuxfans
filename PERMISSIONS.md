# Device Permissions

`/dev/tuxedo_io` is created by the `tuxedo_io` kernel module with permissions
`600 root:root`. `tuxfans` solves this with a udev rule.

## Automatic setup

Every `tuxfans` command that needs device access automatically checks
permissions and offers to install the udev rule if they're wrong.
No separate setup step required.

You can also run it manually:

```bash
tuxfans onboard
```

This installs a udev rule granting world read/write access (`MODE="0666"`)
to `/dev/tuxedo_io`. You'll need to authenticate to write to
`/etc/udev/rules.d/`.

## Manual setup

If `onboard` doesn't work, do it manually:

```bash
sudo sh -c 'echo "SUBSYSTEM==\"tuxedo_io\", KERNEL==\"tuxedo_io\", MODE=\"0666\"" > /etc/udev/rules.d/99-tuxfans.rules'
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Or, for a quick one-shot fix:

```bash
sudo chmod 666 /dev/tuxedo_io
```
