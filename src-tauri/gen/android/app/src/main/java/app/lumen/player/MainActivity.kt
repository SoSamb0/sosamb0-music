package app.lumen.player

import android.graphics.Color
import android.os.Bundle
import android.view.View
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // The interface is always dark: light status/navigation icons whatever the phone's theme.
    enableEdgeToEdge(
      statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
      navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
    )
    super.onCreate(savedInstanceState)
    MediaBridge.init(this)
    window.decorView.setBackgroundColor(Color.rgb(10, 10, 13))

    // The page draws under the status bar and the gesture bar (its background continues there);
    // the interface keeps its controls clear of them using the sizes sent here. Sides (landscape
    // cut-out) and the keyboard are handled by padding the page.
    val content = findViewById<View>(android.R.id.content)
    val density = resources.displayMetrics.density
    ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
      val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
      val ime = insets.getInsets(WindowInsetsCompat.Type.ime())
      view.setPadding(bars.left, 0, bars.right, ime.bottom)
      MediaBridge.setInsets(bars.top / density, if (ime.bottom > 0) 0f else bars.bottom / density)
      WindowInsetsCompat.CONSUMED
    }
  }
}
