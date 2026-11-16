package com.phocast.streamer.camera

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Handler
import android.os.HandlerThread
import android.util.Log
import android.view.Surface
import java.io.OutputStream
import java.nio.ByteBuffer
import java.util.concurrent.CopyOnWriteArrayList

class H264Encoder(
    val width: Int = 1920,
    val height: Int = 1080,
    val fps: Int = 30,
    val bitrate: Int = 4_000_000 // 4 Mbps default
) {
    companion object {
        private const val TAG = "H264Encoder"
        private const val MIME_TYPE = MediaFormat.MIMETYPE_VIDEO_AVC
    }

    private var codec: MediaCodec? = null
    var inputSurface: Surface? = null
        private set

    private var encoderThread: HandlerThread? = null
    private var encoderHandler: Handler? = null

    private val clientStreams = CopyOnWriteArrayList<OutputStream>()
    @Volatile private var isRunning = false

    // Cache latest SPS and PPS headers to immediately send to newly connecting clients
    private var spsPpsHeader: ByteArray? = null

    fun addClient(outputStream: OutputStream) {
        clientStreams.add(outputStream)
        // Send SPS/PPS immediately to the new client so it can initialize its decoder
        spsPpsHeader?.let { header ->
            try {
                outputStream.write(header)
                outputStream.flush()
            } catch (e: Exception) {
                Log.e(TAG, "Error sending SPS/PPS header to client", e)
            }
        }
    }

    fun removeClient(outputStream: OutputStream) {
        clientStreams.remove(outputStream)
    }

    fun hasClients(): Boolean = clientStreams.isNotEmpty()

    fun start() {
        if (isRunning) return
        isRunning = true

        encoderThread = HandlerThread("EncoderThread").apply { start() }
        encoderHandler = Handler(encoderThread!!.looper)

        val format = MediaFormat.createVideoFormat(MIME_TYPE, width, height).apply {
            setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
            setInteger(MediaFormat.KEY_BIT_RATE, bitrate)
            setInteger(MediaFormat.KEY_FRAME_RATE, fps)
            setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1) // 1 second keyframe interval
            // Low-latency encoding flags
            setInteger(MediaFormat.KEY_COMPLEXITY, MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR)
        }

        try {
            codec = MediaCodec.createEncoderByType(MIME_TYPE).apply {
                configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
                inputSurface = createInputSurface()
                start()
            }
        } catch (e: Exception) {
            Log.e(TAG, "Failed to initialize MediaCodec", e)
            isRunning = false
            return
        }

        encoderHandler?.post {
            drainEncoder()
        }
    }

    private fun drainEncoder() {
        val bufferInfo = MediaCodec.BufferInfo()
        val codecInstance = codec ?: return

        while (isRunning) {
            try {
                val outputBufferIndex = codecInstance.dequeueOutputBuffer(bufferInfo, 10_000)
                if (outputBufferIndex >= 0) {
                    val outputBuffer: ByteBuffer? = codecInstance.getOutputBuffer(outputBufferIndex)
                    if (outputBuffer != null && bufferInfo.size > 0) {
                        outputBuffer.position(bufferInfo.offset)
                        outputBuffer.limit(bufferInfo.offset + bufferInfo.size)

                        val chunk = ByteArray(bufferInfo.size)
                        outputBuffer.get(chunk)

                        // Check if this buffer contains codec config (SPS/PPS)
                        if ((bufferInfo.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG) != 0) {
                            spsPpsHeader = chunk
                        }

                        // Broadcast NAL unit to all connected clients
                        broadcastPacket(chunk)
                    }
                    codecInstance.releaseOutputBuffer(outputBufferIndex, false)
                }
            } catch (e: Exception) {
                if (!isRunning) break
                Log.w(TAG, "Drain error: ${e.message}")
            }
        }
    }

    private fun broadcastPacket(chunk: ByteArray) {
        val deadStreams = mutableListOf<OutputStream>()
        for (stream in clientStreams) {
            try {
                stream.write(chunk)
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
        try {
            codec?.stop()
            codec?.release()
        } catch (e: Exception) {
            Log.e(TAG, "Error stopping codec", e)
        }
        codec = null
        inputSurface?.release()
        inputSurface = null

        encoderThread?.quitSafely()
        encoderThread = null
        encoderHandler = null
        clientStreams.clear()
    }
}
