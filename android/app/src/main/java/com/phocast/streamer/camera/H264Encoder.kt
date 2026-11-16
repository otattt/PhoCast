package com.phocast.streamer.camera

import android.util.Log
import java.io.OutputStream
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
    private val headerBuf = ByteArray(4)

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
        // This prevents TCP buffer queue accumulation which causes delay.
        if (!isSending.compareAndSet(false, true)) {
            return
        }

        sendExecutor.execute {
            try {
                val size = jpegBytes.size
                headerBuf[0] = (size ushr 24).toByte()
                headerBuf[1] = (size ushr 16).toByte()
                headerBuf[2] = (size ushr 8).toByte()
                headerBuf[3] = size.toByte()

                val deadStreams = mutableListOf<OutputStream>()

                for (stream in clientStreams) {
                    try {
                        stream.write(headerBuf)
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
