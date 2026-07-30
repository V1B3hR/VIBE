# ASIO Setup Instructions for VIBE DAW

## Why ASIO?
ASIO (Audio Stream Input/Output) is the industry standard for professional audio on Windows:
- **Ultra-low latency** (down to 2-5ms)
- **Direct hardware access** (bypasses Windows audio stack)
- **Used by**: Focusrite, RME, Universal Audio, Steinberg, PreSonus, MOTU, etc.

## Installation Steps

### Option 1: Install ASIO4ALL (Universal Driver)
**Recommended for users without professional audio interface**

1. Download ASIO4ALL from: https://www.asio4all.org/
2. Install ASIO4ALL (it's free and open-source)
3. Restart your computer
4. VIBE will automatically detect ASIO4ALL

### Option 2: Use Your Audio Interface Driver
**For professional audio interfaces**

Most professional audio interfaces come with ASIO drivers:
- **Focusrite**: Install Focusrite Control
- **PreSonus**: Install Universal Control
- **RME**: Install TotalMix FX
- **Universal Audio**: Install UAD Console
- **Steinberg**: Install Yamaha Steinberg USB Driver

After installing your interface's driver, VIBE will detect it automatically.

### Option 3: Build VIBE with ASIO SDK (Developers)

If you're building VIBE from source and want native ASIO support:

1. Download ASIO SDK from Steinberg:
   https://www.steinberg.net/developers/

2. Extract SDK to: `C:\ASIOSDK`

3. Set environment variable:
   ```powershell
   $env:ASIO_SDK_DIR = "C:\ASIOSDK"
   ```

4. Build VIBE with ASIO feature:
   ```powershell
   cargo build --features asio
   ```

## Verifying ASIO in VIBE

1. Launch VIBE
2. Click ⚙️ (Settings) in top bar
3. Select "Audio Settings"
4. In "Audio Driver" dropdown, you should see:
   - **ASIO** (if ASIO4ALL or interface driver is installed)
   - WASAPI (Windows default)

5. Select ASIO and choose your device
6. Set buffer size to 128 or 256 samples for low latency
7. Click "Apply Settings"

## Troubleshooting

### "ASIO not showing in driver list"
- Install ASIO4ALL or your interface's ASIO driver
- Restart VIBE
- Check if other DAWs (FL Studio, Ableton) can see ASIO

### "ASIO driver crashes"
- Try increasing buffer size to 512 samples
- Update your audio interface firmware
- Reinstall ASIO driver

### "High CPU usage with ASIO"
- Increase buffer size (trade latency for stability)
- Close background applications
- Disable Windows audio enhancements

## Latency Comparison

| Driver | Buffer Size | Latency | Use Case |
|--------|-------------|---------|----------|
| ASIO | 64 samples | ~1.5ms | Live performance, recording vocals/guitar |
| ASIO | 128 samples | ~3ms | Music production, MIDI recording |
| ASIO | 256 samples | ~6ms | Mixing, general production |
| WASAPI | 512 samples | ~11ms | Playback, no real-time input needed |

## Recommended Settings

**For Recording/Live Input:**
- Driver: ASIO
- Buffer Size: 128 samples
- Sample Rate: 48000 Hz
- Expected Latency: ~3ms

**For Mixing/Mastering:**
- Driver: ASIO
- Buffer Size: 512 samples
- Sample Rate: 48000 Hz
- Expected Latency: ~11ms (more CPU headroom)

---

**Note**: VIBE will work perfectly fine with WASAPI if you don't have ASIO. ASIO is only critical if you need ultra-low latency for recording or live performance.
