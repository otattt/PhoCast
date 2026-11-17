package com.phocast.streamer.network

import android.util.Log
import com.phocast.streamer.camera.H264Encoder
import java.io.InputStream
import java.io.OutputStream
import java.net.ServerSocket
import java.net.Socket
import kotlin.concurrent.thread

class StreamServer(
    private val port: Int = 5000,
    private val encoder: H264Encoder,
    private val onClientCountChanged: (Int) -> Unit
) {
    companion object {
        private const val TAG = "StreamServer"
    }

    private var serverSocket: ServerSocket? = null
    @Volatile private var isRunning = false
    private var clientCount = 0

    fun start() {
        if (isRunning) return
        isRunning = true

        thread(name = "StreamServerThread") {
            try {
                serverSocket = ServerSocket(port).apply {
                    reuseAddress = true
                }
                Log.i(TAG, "Server started on port $port")

                while (isRunning) {
                    val clientSocket: Socket = serverSocket?.accept() ?: break
                    handleClient(clientSocket)
                }
            } catch (e: Exception) {
                if (isRunning) {
                    Log.e(TAG, "Server socket error", e)
                }
            }
        }
    }

    private fun handleClient(clientSocket: Socket) {
        thread(name = "ClientHandler-${clientSocket.inetAddress.hostAddress}") {
            var rawOut: OutputStream? = null
            try {
                // Disable Nagle's algorithm for immediate packet dispatch
                clientSocket.tcpNoDelay = true
                // Keep send buffer minimal (32 KB) so the kernel never accumulates stale frames.
                // If the link is saturated, writes will block briefly, causing broadcastJpegFrame
                // to cleanly drop backlog frames at sensor rate (true 0-latency).
                clientSocket.sendBufferSize = 32 * 1024
                rawOut = clientSocket.getOutputStream()
                val input: InputStream = clientSocket.getInputStream()

                encoder.addClient(rawOut)
                clientCount++
                onClientCountChanged(clientCount)
                Log.i(TAG, "Client connected: ${clientSocket.inetAddress.hostAddress} (Zero-Buffer Mode)")

                // Monitor connection status by reading input stream until EOF
                val dummy = ByteArray(64)
                while (isRunning && clientSocket.isConnected && !clientSocket.isClosed) {
                    val read = input.read(dummy)
                    if (read == -1) break
                }
            } catch (e: Exception) {
                Log.w(TAG, "Client disconnected or error: ${e.message}")
            } finally {
                try {
                    if (rawOut != null) {
                        encoder.removeClient(rawOut)
                    }
                    clientSocket.close()
                } catch (_: Exception) {}
                clientCount = maxOf(0, clientCount - 1)
                onClientCountChanged(clientCount)
                Log.i(TAG, "Client session closed. Active clients: $clientCount")
            }
        }
    }

    fun stop() {
        isRunning = false
        try {
            serverSocket?.close()
        } catch (e: Exception) {
            Log.e(TAG, "Error closing server socket", e)
        }
        serverSocket = null
        clientCount = 0
        onClientCountChanged(0)
    }
}
