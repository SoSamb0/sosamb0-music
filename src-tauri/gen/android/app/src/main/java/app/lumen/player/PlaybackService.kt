package app.lumen.player

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat
import androidx.core.content.ContextCompat
import androidx.media.app.NotificationCompat.MediaStyle

/**
 * Keeps the app alive while music plays (a "mediaPlayback" foreground service) and shows the
 * media notification. All state comes from [MediaBridge]; buttons go back through it.
 */
class PlaybackService : Service() {
  companion object {
    private const val CHANNEL = "playback"
    private const val ID = 1
    private const val ACTION_REFRESH = "refresh"

    /** Show or update the notification (starts the service if needed). */
    fun refresh(context: Context) {
      val intent = Intent(context, PlaybackService::class.java).setAction(ACTION_REFRESH)
      try {
        if (MediaBridge.info?.state == "playing") ContextCompat.startForegroundService(context, intent)
        else context.startService(intent)
      } catch (_: Exception) {
        // Not allowed to start from the background right now; the next update from the
        // foreground will.
      }
    }

    fun stop(context: Context) {
      context.stopService(Intent(context, PlaybackService::class.java))
    }
  }

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    when (val action = intent?.action) {
      null, ACTION_REFRESH -> {}
      else -> MediaBridge.send(action)
    }
    show()
    return START_NOT_STICKY
  }

  private fun show() {
    val info = MediaBridge.info
    if (info == null || info.state == "stopped" || info.title.isEmpty()) {
      ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_REMOVE)
      stopSelf()
      return
    }
    val manager = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    if (manager.getNotificationChannel(CHANNEL) == null) {
      val channel = NotificationChannel(CHANNEL, "Воспроизведение", NotificationManager.IMPORTANCE_LOW)
      channel.setShowBadge(false)
      manager.createNotificationChannel(channel)
    }
    val notification = build(info)
    if (info.state == "playing") {
      val type = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK else 0
      try {
        ServiceCompat.startForeground(this, ID, notification, type)
      } catch (_: Exception) {
        manager.notify(ID, notification)
      }
    } else {
      // Paused: the notification stays (to resume from it) but can be swiped away.
      ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_DETACH)
      manager.notify(ID, notification)
    }
  }

  private fun button(action: String, icon: Int, title: String, code: Int): NotificationCompat.Action {
    val intent = Intent(this, PlaybackService::class.java).setAction(action)
    val pending = PendingIntent.getService(this, code, intent, PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
    return NotificationCompat.Action(icon, title, pending)
  }

  private fun build(info: MediaBridge.Info): Notification {
    val playing = info.state == "playing"
    val open = PendingIntent.getActivity(
      this, 0, Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
      PendingIntent.FLAG_IMMUTABLE,
    )
    return NotificationCompat.Builder(this, CHANNEL)
      .setSmallIcon(R.drawable.ic_stat_player)
      .setContentTitle(info.title)
      .setContentText(info.artist)
      .setSubText(info.album)
      .setLargeIcon(MediaBridge.cover)
      .setContentIntent(open)
      .setDeleteIntent(button("stop", 0, "", 9).actionIntent)
      .setVisibility(NotificationCompat.VISIBILITY_PUBLIC)
      .setOnlyAlertOnce(true)
      .setShowWhen(false)
      .setOngoing(playing)
      .addAction(button("previous", R.drawable.ic_skip_previous, "Предыдущий", 1))
      .addAction(
        if (playing) button("pause", R.drawable.ic_pause, "Пауза", 2)
        else button("play", R.drawable.ic_play, "Играть", 3)
      )
      .addAction(button("next", R.drawable.ic_skip_next, "Следующий", 4))
      .setStyle(MediaStyle().setMediaSession(MediaBridge.token()).setShowActionsInCompactView(0, 1, 2))
      .build()
  }

  override fun onTaskRemoved(rootIntent: Intent?) {
    // Swiping the app away from recents while paused ends the session.
    if (MediaBridge.info?.state != "playing") stopSelf()
  }
}
