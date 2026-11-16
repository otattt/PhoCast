package com.phocast.streamer.camera

import android.util.Log
import java.io.OutputStream
import java.nio.ByteBuffer
import java.util.concurrent.CopyOnWriteArrayList

class H264Encoder(
    val width: Int = 1280,
    val height: Int = 720,
    val fps: Int = 30,
    val bitrate: Int = 4_000_000
) {
    companion object {
        private const val TAG = "H264Encoder"
    }

    private val clientStreams = CopyOnWriteArrayList<OutputStream>()
    @Volatile private var isRunning = false

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
    }

    fun stop() {
        isRunning = false
        clientStreams.clear()
    }
}
