/**
 * EcosystemCoreActivity — Phase 2 native ecosystem showcase.
 *
 * Demonstrates NATIVE EQUIVALENTS of state management (SignalStore/NgRx
 * concepts), i18n (Transloco/ngx-translate concepts), dynamic forms
 * (Formly/Reactive Forms concepts) and animation (GSAP concepts) — all using
 * real Android widgets. The Rust host is initialized and exercised so the
 * AngNative JNI bridge remains part of the demo.
 *
 * HONESTY: these are native reimplementations of concepts, NOT unchanged
 * Angular ecosystem libraries (those are DOM/TS based and need a JS runtime).
 * No WebView anywhere.
 */
package com.angnative.app

import android.animation.ObjectAnimator
import android.os.Bundle
import android.view.Gravity
import android.view.View
import android.view.animation.OvershootInterpolator
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.core.widget.NestedScrollView

class EcosystemCoreActivity : AppCompatActivity() {

    // ---- State management (SignalStore-like reactive holder) ----
    private data class CoreState(val count: Int = 0, val locale: String = "en")
    private var state = CoreState()
        set(value) {
            field = value
            renderState()
        }

    private lateinit var badge: TextView
    private lateinit var title: TextView
    private lateinit var countLabel: TextView
    private lateinit var statusLabel: TextView
    private lateinit var nameInput: EditText
    private lateinit var emailInput: EditText
    private lateinit var nameError: TextView
    private lateinit var emailError: TextView
    private lateinit var submitResult: TextView

    // Angular-driven native UI (booted from the real AOT bundle in QuickJS).
    private var angularBridge: JniBridge? = null
    private var angularFactory: NativeViewFactory? = null

    private val strings = mapOf(
        "en" to mapOf(
            "badge" to "ANGNATIVE • ECOSYSTEM CORE (NATIVE REIMPLEMENTATION)",
            "title" to "State • i18n • Forms • Animation",
            "state" to "STATE MANAGEMENT (SignalStore concept)",
            "locale" to "INTERNATIONALIZATION (Transloco concept)",
            "forms" to "DYNAMIC FORMS (Formly / Reactive Forms concept)",
            "animation" to "ANIMATION (GSAP concept)",
            "angularLive" to "ANGULAR-DRIVEN NATIVE UI (LIVE AOT BUNDLE)",
            "name" to "Name (required, min 3 chars)",
            "email" to "Email (valid format)",
            "submit" to "Validate & Submit",
            "ok" to "Form valid — submitted natively",
            "rust" to "Rust host nodes",
        ),
        "ur" to mapOf(
            "badge" to "ANGNATIVE • ماحولیاتی بنیاد (مقامی دوبارہ footwear)",
            "title" to "حالت • زبان • فارم • حرکت",
            "state" to "حالت کا انتظام (SignalStore تصور)",
            "locale" to "بین الاقوامی سازی (Transloco تصور)",
            "forms" to "متحرک فارم (Formly / Reactive Forms تصور)",
            "animation" to "حرکت (GSAP تصور)",
            "name" to "نام (ضروری، کم از کم 3 حروف)",
            "email" to "ای میل (درست فارمیٹ)",
            "submit" to "تصدیق کریں اور جمع کریں",
            "ok" to "فارم درست — مقامی طور پر جمع کیا گیا",
            "rust" to "Rust ہوسٹ نوڈز",
        ),
    )

    private fun t(key: String): String =
        strings[state.locale]?.get(key) ?: strings["en"]?.get(key) ?: key

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        supportActionBar?.hide()

        val scroll = NestedScrollView(this).apply {
            setBackgroundColor(0xFFF8FAFC.toInt())
        }
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val p = (20 * resources.displayMetrics.density).toInt()
            setPadding(p, p, p, p)
        }
        scroll.addView(root, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT))

        badge = TextView(this).apply {
            textSize = 12f
            setTextColor(0xFF4338CA.toInt())
            typeface = android.graphics.Typeface.DEFAULT_BOLD
        }
        title = TextView(this).apply {
            textSize = 26f
            setTextColor(0xFF0F172A.toInt())
            typeface = android.graphics.Typeface.DEFAULT_BOLD
        }
        root.addView(badge); root.addView(title)

        // ---- Rust host status (JNI bridge alive) ----
        statusLabel = TextView(this).apply {
            textSize = 13f
            setTextColor(0xFF64748B.toInt())
            setPadding(0, 8, 0, 20)
        }
        root.addView(statusLabel)

        // ---- State section ----
        root.addView(sectionHeader("state"))
        countLabel = TextView(this).apply {
            textSize = 40f
            setTextColor(0xFF2563EB.toInt())
            gravity = Gravity.CENTER
            typeface = android.graphics.Typeface.DEFAULT_BOLD
            setPadding(0, 16, 0, 16)
        }
        root.addView(countLabel)
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
        }
        val minus = nativeButton("−") { state = state.copy(count = state.count - 1) }
        val reset = nativeButton("reset") { state = state.copy(count = 0) }
        val plus = nativeButton("+") { state = state.copy(count = state.count + 1) }
        row.addView(minus); row.addView(reset); row.addView(plus)
        root.addView(row)

        // ---- i18n section ----
        root.addView(sectionHeader("locale"))
        val localeRow = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
        localeRow.addView(nativeButton("English") { state = state.copy(locale = "en") })
        localeRow.addView(nativeButton("اردو") { state = state.copy(locale = "ur") })
        root.addView(localeRow)

        // ---- Forms section ----
        root.addView(sectionHeader("forms"))
        nameInput = nativeInput("name")
        nameError = errorLabel()
        emailInput = nativeInput("email")
        emailError = errorLabel()
        root.addView(nameInput); root.addView(nameError)
        root.addView(emailInput); root.addView(emailError)
        submitResult = TextView(this).apply { textSize = 14f; setTextColor(0xFF16A34A.toInt()) }
        root.addView(nativeButton("submit") { validateForm() })
        root.addView(submitResult)

        // ---- Animation section ----
        root.addView(sectionHeader("animation"))
        val animBox = TextView(this).apply {
            text = "◼"
            textSize = 48f
            setTextColor(0xFF2563EB.toInt())
            gravity = Gravity.CENTER
        }
        root.addView(animBox)
        root.addView(nativeButton("animate") {
            ObjectAnimator.ofFloat(animBox, "scaleX", 1f, 1.4f, 1f).apply {
                duration = 500; interpolator = OvershootInterpolator(2f); start()
            }
            ObjectAnimator.ofFloat(animBox, "rotation", 0f, 360f).apply { duration = 600; start() }
        })

        // ---- Angular-driven native UI (REAL AOT bundle in QuickJS) ----
        root.addView(sectionHeader("angularLive"))
        val angularContainer = FrameLayout(this).apply {
            id = View.generateViewId()
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )
        }
        root.addView(angularContainer)

        setContentView(scroll)
        renderState()
        bootAngularNativeUi(angularContainer)
    }

    /**
     * Boots the real Angular AOT bundle in the Rust-owned QuickJS runtime and
     * materializes its command batches as real Android views. A tap on the
     * Angular "+" button dispatches a "click" event back into QuickJS; the
     * Angular handler updates a signal, emits a new native batch, and the
     * TextView text changes — proving the full round trip in a shipping shell.
     */
    private fun bootAngularNativeUi(container: FrameLayout) {
        try {
            val bridge = JniBridge(this)
            bridge.initHost()
            var factory: NativeViewFactory? = null
            factory = NativeViewFactory(this, container) { nodeId ->
                // Angular's (click) may live on an ancestor host node, so walk
                // up to the root and dispatch for each node in the chain.
                var current: Long? = nodeId
                var hops = 0
                while (current != null && current != 0L && hops < 32) {
                    val invoked = bridge.dispatchNativeEvent(current, "click")
                    android.util.Log.i("AngNative", "Angular tap node=$current -> $invoked listener(s)")
                    if (invoked > 0) break
                    current = factory?.parentOf(current)
                    hops++
                }
            }
            bridge.viewFactory = factory
            angularBridge = bridge
            angularFactory = factory
            // Angular's root component is node 1; attach it to the container.
            factory.setRootNode(1L)

            val bundle = assets.open("angnative-bundle.js").bufferedReader().use { it.readText() }
            val rc = bridge.loadScript(bundle)

            // Layout is produced by the Rust host, not guessed here.
            val w = resources.displayMetrics.widthPixels / resources.displayMetrics.density
            val h = 600f
            val layoutJson = bridge.runLayout(w, h)
            val applied = factory.applyLayoutResult(layoutJson)
            android.util.Log.i("AngNative", "Angular layout applied=$applied json=$layoutJson")
            if (!applied) {
                android.util.Log.w("AngNative", "Angular layout error: ${bridge.lastError()}")
            }

            val plusId = factory.findButtonIdByText("+")
            statusLabel.text = "Angular in Rust: nodes=${bridge.nodeCount()} rc=$rc plusNode=$plusId"

            // Expose ids/text for E2E (uiautomator) assertions.
            container.tag = "angular-native-ui"
            plusId?.let { container.setTag(0x7F000001, it) }
        } catch (e: Exception) {
            statusLabel.text = "Rust host unavailable: ${e.message}"
            android.util.Log.e("AngNative", "Angular boot failed", e)
        }
    }

    private fun sectionHeader(key: String): TextView = TextView(this).apply {
        text = t(key)
        textSize = 15f
        setTextColor(0xFF0F172A.toInt())
        typeface = android.graphics.Typeface.DEFAULT_BOLD
        setPadding(0, 28, 0, 8)
    }

    private fun nativeButton(labelKey: String, click: () -> Unit): Button =
        Button(this).apply {
            text = t(labelKey)
            isAllCaps = false
            setTextColor(0xFFFFFFFF.toInt())
            background = android.graphics.drawable.GradientDrawable().apply {
                cornerRadius = 12f * resources.displayMetrics.density
                setColor(0xFF2563EB.toInt())
            }
            val lp = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT, LinearLayout.LayoutParams.WRAP_CONTENT)
            lp.setMargins(0, 8, (10 * resources.displayMetrics.density).toInt(), 8)
            layoutParams = lp
            setOnClickListener { click() }
        }

    private fun nativeInput(key: String): EditText = EditText(this).apply {
        hint = t(key)
        textSize = 15f
        setPadding(16, 16, 16, 16)
    }

    private fun errorLabel(): TextView = TextView(this).apply {
        textSize = 12f
        setTextColor(0xFFDC2626.toInt())
    }

    private fun validateForm() {
        val name = nameInput.text.toString().trim()
        val email = emailInput.text.toString().trim()
        val nameOk = name.length >= 3
        val emailOk = android.util.Patterns.EMAIL_ADDRESS.matcher(email).matches()
        nameError.text = if (nameOk) "" else if (state.locale == "ur") "نام کم از کم 3 حروف کا ہونا چاہیے" else "Name must be at least 3 characters"
        emailError.text = if (emailOk) "" else if (state.locale == "ur") "درست ای میل درج کریں" else "Enter a valid email"
        submitResult.text = if (nameOk && emailOk) t("ok") else ""
    }

    private fun renderState() {
        if (!::countLabel.isInitialized) return
        badge.text = t("badge")
        title.text = t("title")
        countLabel.text = state.count.toString()
        nameInput.hint = t("name")
        emailInput.hint = t("email")
        // Buttons created before state changes keep their initial locale text;
        // i18n switch re-creates the screen labels where feasible.
        nameError.text = ""
        emailError.text = ""
        submitResult.text = ""
    }

    override fun onDestroy() {
        // Destroy the bridge-owned Host and release its ViewFactory JNI global
        // ref before the activity/views go away, then drop the Kotlin handles.
        angularBridge?.destroyHost()
        angularBridge = null
        angularFactory = null
        super.onDestroy()
    }
}
