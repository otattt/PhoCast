"""
PhoCast Simulator / Mock Streamer
Simulates the PhoCast Android App on your PC for immediate end-to-end testing!
Generates a moving test pattern with timestamp and streams it over TCP port 5000.
"""

import socket
import time
import cv2
import numpy as np

def run_mock_server(host="127.0.0.1", port=5000, width=1280, height=720, fps=30):
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind((host, port))
    server.listen(1)

    print("=" * 60)
    print("      PhoCast Streamer Simulator (Phone Mock Server)")
    print("=" * 60)
    print(f"[*] Simulator listening on {host}:{port}")
    print("[*] Now run 'python phocast_client.py' or 'run_usb.bat' to test connection!")
    print("[*] Press Ctrl+C in this window to stop.")

    while True:
        try:
            client, addr = server.accept()
            client.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            print(f"[✓] Client connected from {addr[0]}:{addr[1]}!")

            # Use OpenCV VideoWriter or simple MJPEG/frame stream
            # For demonstration, we stream standard MPEG / JPEG frames with boundary
            x_pos = 0
            frame_delay = 1.0 / fps

            while True:
                start_loop = time.time()
                # Create test frame
                frame = np.zeros((height, width, 3), dtype=np.uint8)
                frame[:] = (30, 30, 30) # Dark gray background

                # Moving colored bar
                x_pos = (x_pos + 10) % width
                cv2.rectangle(frame, (x_pos, 0), (x_pos + 80, height), (0, 230, 118), -1)

                # Texts
                cv2.putText(frame, "PhoCast Simulator Stream", (60, 100), cv2.FONT_HERSHEY_SIMPLEX, 1.5, (255, 255, 255), 3)
                cv2.putText(frame, f"Time: {time.strftime('%H:%M:%S')}", (60, 160), cv2.FONT_HERSHEY_SIMPLEX, 1.0, (200, 200, 200), 2)
                cv2.putText(frame, f"Resolution: {width}x{height} @ {fps}fps", (60, 210), cv2.FONT_HERSHEY_SIMPLEX, 0.9, (0, 230, 118), 2)
                cv2.putText(frame, "Status: Virtual Camera Pipeline Active", (60, 260), cv2.FONT_HERSHEY_SIMPLEX, 0.9, (100, 200, 255), 2)

                # Encode frame to JPEG
                _, encoded = cv2.imencode(".jpg", frame, [cv2.IMWRITE_JPEG_QUALITY, 80])
                data = encoded.tobytes()

                # Send length header + payload
                header = len(data).to_bytes(4, byteorder='big')
                try:
                    client.sendall(header + data)
                except (BrokenPipeError, ConnectionResetError):
                    print("[!] Client disconnected.")
                    break

                # Maintain target FPS
                elapsed = time.time() - start_loop
                sleep_time = max(0.0, frame_delay - elapsed)
                time.sleep(sleep_time)

        except KeyboardInterrupt:
            print("\n[*] Stopping simulator...")
            break
        except Exception as e:
            print(f"[!] Error: {e}")
            time.sleep(1)

    server.close()

if __name__ == "__main__":
    run_mock_server()
