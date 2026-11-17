package com.phocast.streamer

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Bundle
import android.view.WindowManager
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.content.ContextCompat
import com.phocast.streamer.camera.CameraManager
import com.phocast.streamer.camera.H264Encoder
import com.phocast.streamer.databinding.ActivityMainBinding
import com.phocast.streamer.network.NetworkUtils
import com.phocast.streamer.network.StreamServer

class MainActivity : AppCompatActivity() {

    companion object {
        private const val CAMERA_PERMISSION_CODE = 1001
        private const val DEFAULT_PORT = 5000
    }

    private lateinit var binding: ActivityMainBinding
    private lateinit var encoder: H264Encoder
    private lateinit var cameraManager: CameraManager
    private var streamServer: StreamServer? = null
    private var isStreaming = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        encoder = H264Encoder(width = 1280, height = 720, fps = 30, bitrate = 2_500_000)
        cameraManager = CameraManager(this, this, binding.previewView, encoder)

        setupUI()

        if (allPermissionsGranted()) {
            cameraManager.startCamera {}
        } else {
            ActivityCompat.requestPermissions(
                this,
                arrayOf(Manifest.permission.CAMERA),
                CAMERA_PERMISSION_CODE
            )
        }
    }

    private fun setupUI() {
        val localIp = NetworkUtils.getLocalIpAddress()
        binding.tvWifiInfo.text = "Wi-Fi: $localIp:$DEFAULT_PORT"
        binding.tvUsbInfo.text = "USB: adb forward tcp:$DEFAULT_PORT tcp:$DEFAULT_PORT"

        binding.btnToggleStream.setOnClickListener {
            if (isStreaming) {
                stopStreaming()
            } else {
                startStreaming()
            }
        }

        binding.btnSwitchCamera.setOnClickListener {
            cameraManager.switchCamera()
        }

        binding.btnTorch.setOnClickListener {
            val isOn = cameraManager.toggleTorch()
            val tint = if (isOn) Color.parseColor("#FFD600") else Color.WHITE
            binding.btnTorch.setColorFilter(tint)
        }

        // Screen Off / Keep Cool Mode (AMOLED Black Screen + Disable Local GPU Preview)
        binding.btnDimScreen.setOnClickListener {
            enterKeepCoolMode()
        }

        binding.layoutDimOverlay.setOnClickListener {
            exitKeepCoolMode()
        }

        // Resolution Selector Chips
        binding.chipGroupResolution.setOnCheckedChangeListener { _, checkedId ->
            when (checkedId) {
                R.id.chip480p -> cameraManager.updateConfig(640, 480, cameraManager.targetFps)
                R.id.chip720p -> cameraManager.updateConfig(1280, 720, cameraManager.targetFps)
                R.id.chip1080p -> cameraManager.updateConfig(1920, 1080, cameraManager.targetFps)
            }
        }

        // FPS Selector Chips
        binding.chipGroupFps.setOnCheckedChangeListener { _, checkedId ->
            when (checkedId) {
                R.id.chip30Fps -> cameraManager.updateConfig(cameraManager.targetWidth, cameraManager.targetHeight, 30)
                R.id.chip60Fps -> cameraManager.updateConfig(cameraManager.targetWidth, cameraManager.targetHeight, 60)
            }
        }
    }

    private fun enterKeepCoolMode() {
        cameraManager.setKeepCoolMode(true)
        binding.layoutDimOverlay.visibility = android.view.View.VISIBLE
        binding.previewView.visibility = android.view.View.INVISIBLE
        val lp = window.attributes
        lp.screenBrightness = 0.0f
        window.attributes = lp
    }

    private fun exitKeepCoolMode() {
        cameraManager.setKeepCoolMode(false)
        binding.layoutDimOverlay.visibility = android.view.View.GONE
        binding.previewView.visibility = android.view.View.VISIBLE
        val lp = window.attributes
        lp.screenBrightness = WindowManager.LayoutParams.BRIGHTNESS_OVERRIDE_NONE
        window.attributes = lp
    }

    private fun startStreaming() {
        encoder.start()
        streamServer = StreamServer(DEFAULT_PORT, encoder) { count ->
            runOnUiThread {
                binding.tvClientCount.text = "Connected Clients: $count"
            }
        }
        streamServer?.start()

        isStreaming = true
        binding.tvStatusBadge.text = "STREAMING"
        binding.tvStatusBadge.setBackgroundResource(R.drawable.badge_active)
        binding.btnToggleStream.text = "STOP STREAM"
        binding.btnToggleStream.setBackgroundColor(Color.parseColor("#FF1744"))
        binding.btnToggleStream.setTextColor(Color.WHITE)
        Toast.makeText(this, "PhoCast stream server active on port $DEFAULT_PORT", Toast.LENGTH_SHORT).show()
    }

    private fun stopStreaming() {
        streamServer?.stop()
        streamServer = null
        encoder.stop()

        isStreaming = false
        binding.tvStatusBadge.text = "IDLE"
        binding.tvStatusBadge.setBackgroundResource(R.drawable.badge_idle)
        binding.btnToggleStream.text = "START STREAM"
        binding.btnToggleStream.setBackgroundColor(Color.parseColor("#00E676"))
        binding.btnToggleStream.setTextColor(Color.BLACK)
        binding.tvClientCount.text = "Connected Clients: 0"
    }

    private fun allPermissionsGranted(): Boolean {
        return ContextCompat.checkSelfPermission(
            this,
            Manifest.permission.CAMERA
        ) == PackageManager.PERMISSION_GRANTED
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == CAMERA_PERMISSION_CODE) {
            if (allPermissionsGranted()) {
                cameraManager.startCamera {}
            } else {
                Toast.makeText(this, "Camera permission is required for PhoCast", Toast.LENGTH_LONG).show()
                finish()
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        stopStreaming()
        cameraManager.stopCamera()
    }
}
