"""
PhoCast PC Client - Video Receiver & Virtual Camera Bridge
Connects to PhoCast Android Streamer via USB (ADB) or Wi-Fi,
decodes the live H.264 stream, and feeds it into the Windows Virtual Camera.
"""

import argparse
import subprocess
import sys
import time
import cv2
import numpy as np

def setup_adb_forward(port: int = 5000) -> bool:
    """Attempts to forward local port to Android device using adb."""
    print(f"[*] Setting up ADB port forward: tcp:{port} -> tcp:{port}...")
    try:
        # Check adb devices
        devices_out = subprocess.check_output(["adb", "devices"], stderr=subprocess.STDOUT, text=True)
        lines = [line.strip() for line in devices_out.strip().split("\n")[1:] if line.strip()]
        connected = [l for l in lines if "\tdevice" in l]
        if not connected:
            print("[!] No Android device detected via ADB.")
            print("    Please ensure USB Debugging is enabled and your device is connected via USB.")
            return False

        # Run adb forward
        subprocess.check_call(["adb", "forward", f"tcp:{port}", f"tcp:{port}"])
        print(f"[✓] ADB forward successful on port {port}.")
        return True
    except FileNotFoundError:
        print("[!] 'adb' executable not found in PATH. Please install Android Platform Tools.")
        return False
    except subprocess.CalledProcessError as e:
        print(f"[!] ADB error: {e}")
        return False

def run_client(host: str, port: int, enable_virtual_cam: bool = True):
    stream_url = f"tcp://{host}:{port}"
    print(f"[*] Connecting to PhoCast stream at {stream_url}...")

    # OpenCV VideoCapture with FFmpeg backend can read raw H.264 over TCP directly
    cap = cv2.VideoCapture(stream_url, cv2.CAP_FFMPEG)

    # Low latency buffer settings
    cap.set(cv2.CAP_PROP_BUFFERSIZE, 1)

    if not cap.isOpened():
        print(f"[!] Unable to open stream at {stream_url}.")
        print("    Ensure PhoCast is running on your phone and 'START STREAM' is active.")
        return

    # Read first frame to determine resolution
    ret, frame = cap.read()
    if not ret or frame is None:
        print("[!] Failed to grab initial frame from stream.")
        cap.release()
        return

    height, width, channels = frame.shape
    fps = cap.get(cv2.CAP_PROP_FPS)
    if fps <= 0 or fps > 120:
        fps = 30.0

    print(f"[✓] Stream established: {width}x{height} @ ~{fps:.0f} FPS")

    # Initialize Virtual Camera if requested
    vcam = None
    if enable_virtual_cam:
        try:
            import pyvirtualcam
            vcam = pyvirtualcam.Camera(width=width, height=height, fps=fps, fmt=pyvirtualcam.PixelFormat.BGR)
            print(f"[✓] Virtual camera active: '{vcam.device}'")
            print("    Select this camera in Zoom, Google Meet, Teams, or Discord!")
        except Exception as e:
            print(f"[!] Virtual camera initialization skipped ({e}).")
            print("    Displaying preview window only.")

    print("[*] Streaming live. Press 'Q' or ESC in the preview window to exit.")

    frame_count = 0
    start_time = time.time()

    try:
        while True:
            ret, frame = cap.read()
            if not ret or frame is None:
                print("[!] Stream interrupted or ended.")
                break

            # Send to virtual camera (BGR format)
            if vcam is not None:
                vcam.send(frame)
                vcam.sleep_until_next_frame()

            # Display local preview window
            cv2.imshow("PhoCast Live Preview", frame)
            frame_count += 1

            # Check for user exit
            key = cv2.waitKey(1) & 0xFF
            if key == ord('q') or key == 27:
                break

            # Calculate and display FPS every 5 seconds
            elapsed = time.time() - start_time
            if elapsed >= 5.0:
                current_fps = frame_count / elapsed
                print(f"[*] PhoCast streaming performance: {current_fps:.1f} FPS")
                frame_count = 0
                start_time = time.time()

    except KeyboardInterrupt:
        print("\n[*] Stopping PhoCast client...")
    finally:
        cap.release()
        cv2.destroyAllWindows()
        if vcam is not None:
            vcam.close()
        print("[✓] PhoCast client stopped.")

def main():
    parser = argparse.ArgumentParser(description="PhoCast Desktop Client & Virtual Camera Bridge")
    parser.add_argument("--usb", action="store_true", help="Connect via USB cable (automatically configures ADB port forward)")
    parser.add_argument("--ip", type=str, default="127.0.0.1", help="Target IP address for Wi-Fi mode (default: 127.0.0.1)")
    parser.add_argument("--port", type=int, default=5000, help="Stream port (default: 5000)")
    parser.add_argument("--no-vcam", action="store_true", help="Disable virtual camera, preview window only")

    args = parser.parse_args()

    target_ip = args.ip
    if args.usb:
        target_ip = "127.0.0.1"
        if not setup_adb_forward(args.port):
            sys.exit(1)

    run_client(host=target_ip, port=args.port, enable_virtual_cam=not args.no_vcam)

if __name__ == "__main__":
    main()
