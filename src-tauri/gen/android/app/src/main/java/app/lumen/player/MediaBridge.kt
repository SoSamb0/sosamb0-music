package app.lumen.player

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.media.AudioAttributes
import android.media.AudioFocusRequest
import android.media.AudioManager
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.KeyEvent
import android.support.v4.media.MediaMetadataCompat
import android.support.v4.media.session.MediaSessionCompat
import android.support.v4.media.session.PlaybackStateCompat
import androidx.core.content.ContextCompat
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL
import kotlin.concurrent.thread

/**
 * Android side of playback: the system media session (lock screen, notification shade, headset and
 * Bluetooth buttons, Android Auto), the foreground service that keeps the app alive while music
 * plays, audio focus and "headphones unplugged".
 *
 * The interface owns the queue. It reports what is playing through Rust (`update`, called over JNI),
 * and every button pressed here goes back to it as an action string (`nativeAction`), exactly like
 * the media keys on Windows: "play", "pause", "toggle", "next", "previous", "stop", "seek:<sec>".
 */
object MediaBridge {
  data class Info(
    val title: String,
    val artist: String,
    val album: String,
    val coverUrl: String,
    val duration: Double,
    val position: Double,
    /** "playing", "paused" or "stopped". */
    val state: String,
  )

  private val main = Handler(Looper.getMainLooper())
  private lateinit var app: Context
  private var session: MediaSessionCompat? = null
  private var started = false

  var info: Info? = null
    private set
  var cover: Bitmap? = null
    private set
  private var coverUrl = ""
  private var lastMeta = ""

  /** Called by Rust when the user acts on the system controls. */
  @JvmStatic external fun nativeAction(action: String)

  /** Status bar and gesture bar heights in CSS pixels, for the interface's safe areas. */
  @JvmStatic private external fun nativeInsets(top: Float, bottom: Float)

  fun setInsets(top: Float, bottom: Float) {
    try {
      nativeInsets(top, bottom)
    } catch (_: Throwable) {
    }
  }

  /** Hands Rust a reference to this class so it can call [update] from its own threads. */
  @JvmStatic private external fun nativeInit()

  fun init(context: Context) {
    // The activity can be recreated; the session belongs to the process.
    if (session != null) return
    app = context.applicationContext
    System.loadLibrary("lumen_lib")
    nativeInit()
    val s = MediaSessionCompat(app, "SoSamb0 Music")
    s.setCallback(object : MediaSessionCompat.Callback() {
      override fun onPlay() = send("play")
      override fun onPause() = send("pause")
      override fun onStop() = send("stop")
      override fun onSkipToNext() = send("next")
      override fun onSkipToPrevious() = send("previous")
      override fun onSeekTo(pos: Long) = send("seek:${pos / 1000.0}")

      // Headset and Bluetooth keys. The library's own handling drops separate PLAY and PAUSE
      // keys (Bluetooth headphones send those rather than PLAY_PAUSE), so map them here.
      override fun onMediaButtonEvent(intent: Intent): Boolean {
        @Suppress("DEPRECATION")
        val event = intent.getParcelableExtra<KeyEvent>(Intent.EXTRA_KEY_EVENT)
          ?: return super.onMediaButtonEvent(intent)
        val action = when (event.keyCode) {
          KeyEvent.KEYCODE_MEDIA_PLAY -> "play"
          KeyEvent.KEYCODE_MEDIA_PAUSE -> "pause"
          KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE, KeyEvent.KEYCODE_HEADSETHOOK -> "toggle"
          KeyEvent.KEYCODE_MEDIA_NEXT -> "next"
          KeyEvent.KEYCODE_MEDIA_PREVIOUS -> "previous"
          KeyEvent.KEYCODE_MEDIA_STOP -> "stop"
          else -> return super.onMediaButtonEvent(intent)
        }
        if (event.action == KeyEvent.ACTION_DOWN && event.repeatCount == 0) send(action)
        return true
      }
    })
    s.setSessionActivity(
      android.app.PendingIntent.getActivity(
        app, 0, Intent(app, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
        android.app.PendingIntent.FLAG_IMMUTABLE,
      )
    )
    session = s
  }

  fun token(): MediaSessionCompat.Token? = session?.sessionToken

  fun send(action: String) {
    try {
      nativeAction(action)
    } catch (_: Throwable) {
    }
  }

  /** From Rust (any thread): the current track and state as JSON. */
  @JvmStatic
  fun update(json: String) {
    val o = try { JSONObject(json) } catch (_: Exception) { return }
    val next = Info(
      o.optString("title"), o.optString("artist"), o.optString("album"), o.optString("coverUrl"),
      o.optDouble("duration", 0.0), o.optDouble("position", 0.0), o.optString("state", "stopped"),
    )
    main.post { apply(next) }
  }

  private fun apply(next: Info) {
    val s = session ?: return
    val prev = info
    info = next
    val hasTrack = next.title.isNotEmpty() && next.state != "stopped"

    val meta = "${next.title}\u0000${next.artist}\u0000${next.album}\u0000${next.duration}"
    val metaChanged = meta != lastMeta
    if (metaChanged) {
      lastMeta = meta
      if (next.coverUrl != coverUrl) {
        coverUrl = next.coverUrl
        cover = null
        loadCover(next.coverUrl)
      }
      s.setMetadata(metadata(next))
    }

    val playing = next.state == "playing"
    val actions = PlaybackStateCompat.ACTION_PLAY or PlaybackStateCompat.ACTION_PAUSE or
      PlaybackStateCompat.ACTION_PLAY_PAUSE or PlaybackStateCompat.ACTION_SKIP_TO_NEXT or
      PlaybackStateCompat.ACTION_SKIP_TO_PREVIOUS or PlaybackStateCompat.ACTION_SEEK_TO or
      PlaybackStateCompat.ACTION_STOP
    s.setPlaybackState(
      PlaybackStateCompat.Builder()
        .setActions(actions)
        .setState(
          when {
            !hasTrack -> PlaybackStateCompat.STATE_STOPPED
            playing -> PlaybackStateCompat.STATE_PLAYING
            else -> PlaybackStateCompat.STATE_PAUSED
          },
          (next.position * 1000).toLong(),
          if (playing) 1f else 0f,
          SystemClock.elapsedRealtime(),
        )
        .build()
    )
    s.isActive = hasTrack

    if (playing) gainFocus() else if (!hasTrack) abandonFocus()
    noisy(playing)

    // The service (and its notification) lives while there is a track; it stays in the
    // foreground only while playing, like other music players.
    if (hasTrack) {
      if (!started || prev?.state != next.state || metaChanged) PlaybackService.refresh(app)
      started = true
    } else if (started) {
      started = false
      PlaybackService.stop(app)
    }
  }

  private fun metadata(i: Info): MediaMetadataCompat =
    MediaMetadataCompat.Builder()
      .putString(MediaMetadataCompat.METADATA_KEY_TITLE, i.title)
      .putString(MediaMetadataCompat.METADATA_KEY_ARTIST, i.artist)
      .putString(MediaMetadataCompat.METADATA_KEY_ALBUM, i.album)
      .putLong(MediaMetadataCompat.METADATA_KEY_DURATION, (i.duration * 1000).toLong())
      .also { b -> cover?.let { b.putBitmap(MediaMetadataCompat.METADATA_KEY_ALBUM_ART, it) } }
      .build()

  private fun loadCover(url: String) {
    if (url.isEmpty()) return
    thread(name = "lumen-cover") {
      val bitmap = try {
        val c = URL(url).openConnection() as HttpURLConnection
        c.connectTimeout = 8000
        c.readTimeout = 8000
        c.inputStream.use { BitmapFactory.decodeStream(it) }
      } catch (_: Exception) {
        null
      }
      main.post {
        if (url != coverUrl || bitmap == null) return@post
        cover = bitmap
        info?.let { session?.setMetadata(metadata(it)) }
        PlaybackService.refresh(app)
      }
    }
  }

  // ------------------------------------------------------------------ audio focus

  private val audio by lazy { app.getSystemService(Context.AUDIO_SERVICE) as AudioManager }
  private var focus: AudioFocusRequest? = null
  /** Paused by a phone call or another player, to be resumed when it ends. */
  private var resumeOnGain = false

  private val focusListener = AudioManager.OnAudioFocusChangeListener { change ->
    when (change) {
      AudioManager.AUDIOFOCUS_LOSS -> {
        resumeOnGain = false
        send("pause")
      }
      AudioManager.AUDIOFOCUS_LOSS_TRANSIENT -> {
        resumeOnGain = info?.state == "playing"
        send("pause")
      }
      AudioManager.AUDIOFOCUS_GAIN -> {
        if (resumeOnGain) send("play")
        resumeOnGain = false
      }
      // Notifications over music: the system lowers our volume by itself.
    }
  }

  private fun gainFocus() {
    if (focus != null) return
    val request = AudioFocusRequest.Builder(AudioManager.AUDIOFOCUS_GAIN)
      .setAudioAttributes(
        AudioAttributes.Builder()
          .setUsage(AudioAttributes.USAGE_MEDIA)
          .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
          .build()
      )
      .setOnAudioFocusChangeListener(focusListener, main)
      .build()
    audio.requestAudioFocus(request)
    focus = request
  }

  private fun abandonFocus() {
    focus?.let { audio.abandonAudioFocusRequest(it) }
    focus = null
    resumeOnGain = false
  }

  // ------------------------------------------------------------------ headphones unplugged

  private var noisyRegistered = false
  private val noisyReceiver = object : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
      if (intent.action == AudioManager.ACTION_AUDIO_BECOMING_NOISY) send("pause")
    }
  }

  private fun noisy(listen: Boolean) {
    if (listen == noisyRegistered) return
    if (listen) {
      ContextCompat.registerReceiver(
        app, noisyReceiver, IntentFilter(AudioManager.ACTION_AUDIO_BECOMING_NOISY),
        ContextCompat.RECEIVER_NOT_EXPORTED,
      )
    } else {
      app.unregisterReceiver(noisyReceiver)
    }
    noisyRegistered = listen
  }
}
