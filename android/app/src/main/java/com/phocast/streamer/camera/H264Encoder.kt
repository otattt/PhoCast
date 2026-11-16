package com.phocast.streamer.camera

import android.util.Log
import java.io.OutputStream
import java.nio.ByteBuffer
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean

class H264Encoder(
    val width: Int = 1920,
    val height: Int = 1080,
    val fps: Int = 30,
    val bitrate: Int = 4_000_000
) {
    companion object {
        private const val TAG = "H264Encoder"
    }

    private val clientStreams = CopyOnWriteArrayList<OutputStream>()
    @Volatile private var isRunning = false
    private val isSending = AtomicBoolean(false)
    private val sendExecutor = Executors.newSingleThreadExecutor()

    fun addClient(outputStream: OutputStream) {
        clientStreams.add(outputStream)
        Log.i(TAG, "New client registered. Total active: ${clientStreams.size}")
    }

    fun removeClient(outputStream: OutputStream) {
        clientStreams.remove(outputStream)
        Log.i(TAG, "Client removed. Remaining active: ${clientStreams.size}")
    }

    fun hasClients(): Boolean = clientStreams.isNotEmpty()

    fun start() {
        isRunning = true
    }

    fun broadcastJpegFrame(jpegBytes: ByteArray) {
        if (!isRunning || clientStreams.isEmpty()) return

        // Zero-latency drop-frame policy:
        // If previous frame is still being written over the wire, drop this frame immediately.
        // This prevents TCP buffer queue accumulation which causes 200ms-500ms delay.
        if (!isSending.compareAndSet(false, true)) {
            return
        }

        sendExecutor.execute {
            try {
                // 4-byte big-endian payload length header
                val header = ByteBuffer.allocate(4).putInt(jpegBytes.size).array()
                val deadStreams = mutableListOf<OutputStream>()

                for (stream in clientStreams) {
                    try {
                        stream.write(header)
                        stream.write(jpegBytes)
                        stream.flush()
                    } catch (e: Exception) {
                        deadStreams.add(stream)
                    }
                }

                if (deadStreams.isNotEmpty()) {
                    clientStreams.removeAll(deadStreams)
                }
            } finally {
                isSending.set(false)
            }
        }
    }

    fun stop() {
        isRunning = false
        clientStreams.clear()
    }
}
