package app.tubescript.android

import android.os.Bundle
import android.view.View
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.updatePadding

class MainActivity : TauriActivity() {
  // Donanım geri tuşu: önce WebView geçmişinde geri git, kök sayfada uygulamadan çık.
  override val handleBackNavigation: Boolean = true

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    // Koyu tema: sistem çubuklarındaki simgeler beyaz olsun.
    WindowCompat.getInsetsController(window, window.decorView).apply {
      isAppearanceLightStatusBars = false
      isAppearanceLightNavigationBars = false
    }

    // Edge-to-edge zorunlu olduğu için WebView'i sistem çubuklarının altına alma:
    // içerik köküne status bar / nav bar / çentik kadar boşluk ver.
    val root = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(root) { v, insets ->
      val bars = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or
          WindowInsetsCompat.Type.displayCutout() or
          WindowInsetsCompat.Type.ime()
      )
      v.updatePadding(bars.left, bars.top, bars.right, bars.bottom)
      insets
    }
  }
}
