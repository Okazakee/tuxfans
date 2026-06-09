# Device Permissions

`/dev/tuxedo_io` is created by the `tuxedo_io` kernel module with permissions
`600 root:root`. `tuxfans` solves this with a udev rule.

## Automatic setup

```bash
tuxfans onboard
```

This installs a udev rule granting access to the `plugdev` group, then
reloads udev. You'll need to answer the `pkexec` password prompt to write
to `/etc/udev/rules.d/`.

## Manual setup

If `onboard` doesn't work, do it manually:

```bash
sudo sh -c 'echo "SUBSYSTEM==\"tuxedo_io\", KERNEL==\"tuxedo_io\", MODE=\"0660\", GROUP=\"plugdev\"" > /etc/udev/rules.d/99-tuxfans.rules'
sudo udevadm control --reload-rules
sudo udevadm trigger
```

Make sure your user is in the `plugdev` group:

```bash
groups $USER | grep plugdev || sudo usermod -aG plugdev $USER
```

Log out and back in for group changes to take effect.
