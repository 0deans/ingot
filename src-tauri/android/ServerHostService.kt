package org.ingot.server

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import androidx.core.app.NotificationCompat

/**
 * Android Foreground Service to host Minecraft servers with guaranteed persistence.
 * Prevents OS and OEM background killers from stopping server processes.
 * Holds CPU PARTIAL_WAKE_LOCK and high-performance Wi-Fi lock during server uptime.
 */
class ServerHostService : Service() {

    private var wakeLock: PowerManager.WakeLock? = null
    private var wifiLock: WifiManager.WifiLock? = null

    companion object {
        const val CHANNEL_ID = "ingot_server_host_channel"
        const val NOTIFICATION_ID = 25565
        const val ACTION_START = "org.ingot.server.START"
        const val ACTION_STOP = "org.ingot.server.STOP"
        const val ACTION_UPDATE_STATUS = "org.ingot.server.UPDATE_STATUS"
        const val EXTRA_SERVER_NAME = "server_name"
        const val EXTRA_STATUS = "status"
        const val EXTRA_PLAYERS = "players"
        const val EXTRA_TUNNEL_URL = "tunnel_url"

        fun start(context: Context, serverName: String, status: String = "Starting") {
            val intent = Intent(context, ServerHostService::class.java).apply {
                action = ACTION_START
                putExtra(EXTRA_SERVER_NAME, serverName)
                putExtra(EXTRA_STATUS, status)
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            val intent = Intent(context, ServerHostService::class.java).apply {
                action = ACTION_STOP
            }
            context.stopService(intent)
        }
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        acquireLocks()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val action = intent?.action ?: ACTION_START
        if (action == ACTION_STOP) {
            stopSelf()
            return START_NOT_STICKY
        }

        val serverName = intent?.getStringExtra(EXTRA_SERVER_NAME) ?: "Minecraft Server"
        val status = intent?.getStringExtra(EXTRA_STATUS) ?: "Running"
        val players = intent?.getStringExtra(EXTRA_PLAYERS) ?: "0"
        val tunnelUrl = intent?.getStringExtra(EXTRA_TUNNEL_URL)

        val notification = buildNotification(serverName, status, players, tunnelUrl)
        startForeground(NOTIFICATION_ID, notification)

        return START_STICKY
    }

    private fun acquireLocks() {
        // 1. Acquire CPU PARTIAL_WAKE_LOCK to prevent CPU sleep when screen turns off
        val powerManager = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = powerManager.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "Ingot::ServerWakeLock"
        ).apply {
            setReferenceCounted(false)
            acquire()
        }

        // 2. Acquire High-Performance Wi-Fi Lock to prevent radio throttling
        val wifiManager = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
        wifiLock = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            wifiManager.createWifiLock(
                WifiManager.WIFI_MODE_FULL_LOW_LATENCY,
                "Ingot::ServerWifiLock"
            )
        } else {
            @Suppress("DEPRECATION")
            wifiManager.createWifiLock(
                WifiManager.WIFI_MODE_FULL_HIGH_PERF,
                "Ingot::ServerWifiLock"
            )
        }.apply {
            setReferenceCounted(false)
            acquire()
        }
    }

    private fun releaseLocks() {
        wakeLock?.let {
            if (it.isHeld) it.release()
        }
        wifiLock?.let {
            if (it.isHeld) it.release()
        }
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Ingot Server Hosting",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Keeps Minecraft server and reverse proxy active in background"
                setShowBadge(false)
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(
        serverName: String,
        status: String,
        players: String,
        tunnelUrl: String?
    ): Notification {
        val launchIntent = packageManager.getLaunchIntentForPackage(packageName)
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            launchIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val stopIntent = Intent(this, ServerHostService::class.java).apply {
            action = ACTION_STOP
        }
        val stopPendingIntent = PendingIntent.getService(
            this,
            1,
            stopIntent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val contentText = when {
            status.equals("sleeping", ignoreCase = true) -> "💤 Standby (Sleeping) • Connect to wake"
            tunnelUrl != null -> "Online: $players players • Tunnel: $tunnelUrl"
            else -> "Online: $players players • Local Port: 25565"
        }

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Ingot: $serverName ($status)")
            .setContentText(contentText)
            .setSmallIcon(android.R.drawable.ic_dialog_info)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .addAction(android.R.drawable.ic_menu_close_clear_cancel, "Stop Server", stopPendingIntent)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setCategory(NotificationCompat.CATEGORY_SERVICE)
            .build()
    }

    override fun onDestroy() {
        releaseLocks()
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null
}
