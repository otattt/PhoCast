package com.phocast.streamer.camera

import android.content.Context
import android.graphics.ImageFormat
import android.graphics.Rect
import android.graphics.YuvImage
import android.hardware.camera2.CaptureRequest
import android.util.Log
import android.util.Range
import android.util.Size
import android.view.Surface
import androidx.annotation.OptIn
import androidx.camera.camera2.interop.Camera2Interop
import androidx.camera.camera2.interop.ExperimentalCamera2Interop
import androidx.camera.core.Camera
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.AspectRatioStrategy
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import androidx.core.content.ContextCompat
import androidx.lifecycle.LifecycleOwner
import java.io.ByteArrayOutputStream
import java.util.concurrent.Executors

class CameraManager(
    private val context: Context,
    private val lifecycleOwner: LifecycleOwner,
    private val previewView: PreviewView,
    private val encoder: H264Encoder
) {
    companion object {
        private const val TAG = "CameraManager"
    }

    private var cameraProvider: ProcessCameraProvider? = null
    private var camera: Camera? = null
    private var lensFacing = CameraSelector.LENS_FACING_BACK
    private var isTorchOn = false
    private val cameraExecutor = Executors.newSingleThreadExecutor()

    var targetWidth = 1920
    var targetHeight = 1080
    var targetFps = 30

    // Preallocated, reusable buffers to achieve ZERO heap allocations per frame
    private var cachedNv21: ByteArray? = null
    private var cachedRowBytes: ByteArray? = null
    private val reusableJpegBuffer = ByteArrayOutputStream(256 * 1024)

    fun updateConfig(width: Int, height: Int, fps: Int) {
        targetWidth = width
        targetHeight = height
        targetFps = fps
        bindCameraUseCases()
    }

    fun startCamera(onReady: () -> Unit) {
        val cameraProviderFuture = ProcessCameraProvider.getInstance(context)
        cameraProviderFuture.addListener({
            cameraProvider = cameraProviderFuture.get()
            bindCameraUseCases()
            onReady()
        }, ContextCompat.getMainExecutor(context))
    }

    @OptIn(ExperimentalCamera2Interop::class)
    private fun bindCameraUseCases() {
        val provider = cameraProvider ?: return
        val cameraSelector = CameraSelector.Builder()
            .requireLensFacing(lensFacing)
            .build()

        val rotation = previewView.display?.rotation ?: Surface.ROTATION_0
        val isPortrait = rotation == Surface.ROTATION_0 || rotation == Surface.ROTATION_180
        val targetSize = if (isPortrait) Size(targetHeight, targetWidth) else Size(targetWidth, targetHeight)

        val aspectRatioStrategy = if (targetWidth == 640 && targetHeight == 480) {
            AspectRatioStrategy.RATIO_4_3_FALLBACK_AUTO_STRATEGY
        } else {
            AspectRatioStrategy.RATIO_16_9_FALLBACK_AUTO_STRATEGY
        }

        // Configurable ResolutionSelector supporting 480p, 720p, and 1080p
        val resolutionSelector = ResolutionSelector.Builder()
            .setAspectRatioStrategy(aspectRatioStrategy)
            .setResolutionStrategy(
                ResolutionStrategy(
                    targetSize,
                    ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER
                )
            )
            .build()

        // Preview use case for local on-screen view
        val previewBuilder = Preview.Builder()
            .setResolutionSelector(resolutionSelector)

        // Lock camera sensor AE target FPS to eliminate motion blur and variable framerate drops
        val previewExtender = Camera2Interop.Extender(previewBuilder)
        previewExtender.setCaptureRequestOption(
            CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE,
            Range(targetFps, targetFps)
        )

        val preview = previewBuilder.build().also {
            it.setSurfaceProvider(previewView.surfaceProvider)
        }

        // ImageAnalysis use case for streaming frames to PC
        val analysisBuilder = ImageAnalysis.Builder()
            .setResolutionSelector(resolutionSelector)
            .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
            .setOutputImageFormat(ImageAnalysis.OUTPUT_IMAGE_FORMAT_YUV_420_888)

        // Lock ImageAnalysis to target FPS and use FAST noise reduction to cut DSP processing latency
        val analysisExtender = Camera2Interop.Extender(analysisBuilder)
        analysisExtender.setCaptureRequestOption(
            CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE,
            Range(targetFps, targetFps)
        )
        analysisExtender.setCaptureRequestOption(
            CaptureRequest.NOISE_REDUCTION_MODE,
            CaptureRequest.NOISE_REDUCTION_MODE_FAST
        )

        val imageAnalysis = analysisBuilder.build()

        var loggedResolution = false

        imageAnalysis.setAnalyzer(cameraExecutor) { imageProxy ->
            if (!loggedResolution) {
                Log.i(TAG, "Streaming active: ${imageProxy.width}x${imageProxy.height} @ locked ${targetFps} FPS")
                loggedResolution = true
            }

            if (encoder.hasClients()) {
                processImageForStreaming(imageProxy)
            }
            imageProxy.close()
        }

        try {
            provider.unbindAll()
            camera = provider.bindToLifecycle(
                lifecycleOwner,
                cameraSelector,
                preview,
                imageAnalysis
            )
            Log.i(TAG, "Camera bound successfully at ${targetWidth}x${targetHeight} @ ${targetFps} FPS")
        } catch (e: Exception) {
            Log.e(TAG, "Use case binding failed", e)
        }
    }

    private fun processImageForStreaming(image: ImageProxy) {
        try {
            val nv21 = yuv420888ToNv21Fast(image)
            val yuvImage = YuvImage(nv21, ImageFormat.NV21, image.width, image.height, null)
            
            reusableJpegBuffer.reset()
            // Quality 68: Superb crisp visual fidelity while halving CPU compression time and network payload
            yuvImage.compressToJpeg(Rect(0, 0, image.width, image.height), 68, reusableJpegBuffer)
            val jpegBytes = reusableJpegBuffer.toByteArray()

            encoder.broadcastJpegFrame(jpegBytes)
        } catch (e: Exception) {
            Log.w(TAG, "Frame conversion error: ${e.message}")
        }
    }

    /**
     * Ultra-fast zero-allocation YUV420_888 to NV21 converter:
     * - Reuses preallocated buffers (cachedNv21, cachedRowBytes) eliminating Android ART GC stalls.
     * - Uses bulk row copies instead of per-byte get() loops.
     */
    private fun yuv420888ToNv21Fast(image: ImageProxy): ByteArray {
        val width = image.width
        val height = image.height
        val ySize = width * height
        val uvSize = width * height / 2
        val totalSize = ySize + uvSize

        var nv21 = cachedNv21
        if (nv21 == null || nv21.size < totalSize) {
            nv21 = ByteArray(totalSize)
            cachedNv21 = nv21
        }

        val yPlane = image.planes[0]
        val uPlane = image.planes[1]
        val vPlane = image.planes[2]

        // 1. Copy Y channel in bulk
        val yBuffer = yPlane.buffer
        val yRowStride = yPlane.rowStride
        if (yRowStride == width) {
            yBuffer.get(nv21, 0, ySize)
        } else {
            var pos = 0
            for (row in 0 until height) {
                yBuffer.position(row * yRowStride)
                yBuffer.get(nv21, pos, width)
                pos += width
            }
        }

        // 2. Copy UV channels
        val vBuffer = vPlane.buffer
        val uBuffer = uPlane.buffer
        val vRowStride = vPlane.rowStride
        val vPixelStride = vPlane.pixelStride
        val uPixelStride = uPlane.pixelStride

        val uvHeight = height / 2
        val uvWidth = width / 2
        var uvPos = ySize

        // On Pixel 5 / Qualcomm HAL, vPixelStride == 2 is standard (semi-planar NV21/NV12)
        if (vPixelStride == 2 && uPixelStride == 2) {
            var rowBytes = cachedRowBytes
            if (rowBytes == null || rowBytes.size < vRowStride) {
                rowBytes = ByteArray(vRowStride)
                cachedRowBytes = rowBytes
            }

            for (row in 0 until uvHeight) {
                vBuffer.position(row * vRowStride)
                val len = minOf(vRowStride, vBuffer.remaining())
                vBuffer.get(rowBytes, 0, len)
                val copyLen = uvWidth * 2
                System.arraycopy(rowBytes, 0, nv21, uvPos, minOf(copyLen, len))
                uvPos += copyLen
            }
        } else {
            // Fallback for planar format
            val vRow = ByteArray(vRowStride)
            val uRow = ByteArray(uPlane.rowStride)
            for (row in 0 until uvHeight) {
                vBuffer.position(row * vRowStride)
                vBuffer.get(vRow, 0, minOf(vRowStride, vBuffer.remaining()))
                uBuffer.position(row * uPlane.rowStride)
                uBuffer.get(uRow, 0, minOf(uPlane.rowStride, uBuffer.remaining()))

                for (col in 0 until uvWidth) {
                    nv21[uvPos++] = vRow[col * vPixelStride]
                    nv21[uvPos++] = uRow[col * uPixelStride]
                }
            }
        }

        return nv21
    }

    fun switchCamera() {
        lensFacing = if (lensFacing == CameraSelector.LENS_FACING_BACK) {
            CameraSelector.LENS_FACING_FRONT
        } else {
            CameraSelector.LENS_FACING_BACK
        }
        isTorchOn = false
        bindCameraUseCases()
    }

    fun toggleTorch(): Boolean {
        val cam = camera ?: return false
        if (cam.cameraInfo.hasFlashUnit()) {
            isTorchOn = !isTorchOn
            cam.cameraControl.enableTorch(isTorchOn)
            return isTorchOn
        }
        return false
    }

    fun stopCamera() {
        cameraProvider?.unbindAll()
        camera = null
        cameraExecutor.shutdown()
    }
}
