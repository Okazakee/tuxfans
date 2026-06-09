# Troubleshooting

## CPU stuck at 600 MHz (PROCHOT latch)

On AMD TUXEDO / Clevo laptops, sustained heavy CPU load (e.g. a full Rust build pegging all cores) can trigger PROCHOT thermal throttling. The EC firmware sometimes **latches** this state, keeping the CPU locked at ~600 MHz even after temperatures have dropped to normal.

### Fix

Suspend/resume clears the latched PROCHOT flag in the EC:

```bash
sudo systemctl suspend && sleep 2 && sudo rtcwake -m no -s 1
```

Or just close the lid and open it again.

After resume, reapply ryzenadj if you use it:

```bash
sudo systemctl restart ryzenadj
```
