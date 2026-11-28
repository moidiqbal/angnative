package com.angnative.app

import android.graphics.Rect
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.TextView
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import java.util.concurrent.atomic.AtomicReference

/**
 * Device-side regression for the flagship presentation bug.
 *
 * It boots the real AOT bundle, runs the Rust layout pass on the real shadow
 * tree, applies the result through [NativeViewFactory], and asserts that the
 * four Angular counter controls occupy distinct, non-overlapping screen rects.
 *
 * Added in the layout-fix round and executed on physical OnePlus `b6640c8f`
 * after the demo text-binding fix; see `docs/evidence/ANGULAR-COUNTER-ONEPLUS.md`.
 */
@RunWith(AndroidJUnit4::class)
class AngularCounterLayoutTest {

    @Test
    fun angularCounterViewsDoNotOverlap() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val failure = AtomicReference<Throwable>()

        instrumentation.runOnMainSync {
            val bridge = JniBridge(instrumentation.targetContext)
            var initialized = false
            try {
                val container = FrameLayout(instrumentation.targetContext).apply {
                    layoutParams = FrameLayout.LayoutParams(
                        FrameLayout.LayoutParams.MATCH_PARENT,
                        FrameLayout.LayoutParams.MATCH_PARENT,
                    )
                }
                bridge.initHost()
                initialized = true

                var factory: NativeViewFactory? = null
                factory = NativeViewFactory(instrumentation.targetContext, container)
                bridge.viewFactory = factory
                factory.setRootNode(1L)

                val bundle = instrumentation.targetContext.assets
                    .open("angnative-bundle.js").bufferedReader().use { it.readText() }
                assertEquals("Angular bundle must load", 0, bridge.loadScript(bundle))

                val density = instrumentation.targetContext.resources.displayMetrics.density
                val widthDp = instrumentation.targetContext.resources.displayMetrics.widthPixels / density
                val layoutJson = bridge.runLayout(widthDp, 600f)
                assertFalse("layout returned an error: $layoutJson", layoutJson.contains("\"error\""))
                assertTrue(
                    "layout result must apply: ${bridge.lastError()}",
                    factory.applyLayoutResult(layoutJson),
                )

                // Establish view sizes so WRAP_CONTENT labels have real bounds.
                container.measure(
                    View.MeasureSpec.makeMeasureSpec(1080, View.MeasureSpec.EXACTLY),
                    View.MeasureSpec.makeMeasureSpec(1920, View.MeasureSpec.EXACTLY),
                )
                container.layout(0, 0, container.measuredWidth, container.measuredHeight)

                val count = findTextView(container, "Count: 0")
                val minus = findTextViewWhere(container) { it == "−" || it == "-" }
                val reset = findTextView(container, "reset")
                val plus = findTextView(container, "+")
                assertNotNull("Count label missing", count)
                assertNotNull("Minus button label missing", minus)
                assertNotNull("Reset button label missing", reset)
                assertNotNull("Plus button label missing", plus)

                // Exercise the complete native event path: Kotlin dispatches
                // the native + node through JNI, RuntimeBridge routes the
                // listenerId/payload to na_onEvent, Angular updates its
                // signal, and the resulting SetText op updates the real view.
                val plusNodeId = bridge.snapshot().nodes
                    .firstOrNull { it.textContent == "+" }
                    ?.id
                    ?: throw AssertionError("Angular '+' node missing from host snapshot")
                val invoked = bridge.dispatchNativeEvent(plusNodeId, "click")
                assertTrue("Expected an Angular click listener, invoked=$invoked", invoked > 0)
                assertEquals(
                    "Angular signal update must reach the native TextView",
                    "Count: 1",
                    findTextView(container, "Count: 1")?.text?.toString(),
                )

                val rects = listOf(
                    "Count" to screenRect(findTextView(container, "Count: 1")!!),
                    "minus" to screenRect(minus!!),
                    "reset" to screenRect(reset!!),
                    "plus" to screenRect(plus!!),
                )
                rects.forEach { (label, rect) ->
                    assertFalse("$label has empty bounds: $rect", rect.isEmpty())
                }
                for (i in rects.indices) {
                    for (j in (i + 1) until rects.size) {
                        assertFalse(
                            "${rects[i].first} overlaps ${rects[j].first}: " +
                                "${rects[i].second} / ${rects[j].second}",
                            Rect.intersects(rects[i].second, rects[j].second),
                        )
                    }
                }
            } catch (t: Throwable) {
                failure.set(t)
            } finally {
                if (initialized) bridge.destroyHost()
            }
        }

        failure.get()?.let { throw AssertionError("Angular counter layout assertion failed", it) }
    }

    private fun screenRect(view: View): Rect {
        val rect = Rect()
        check(view.getGlobalVisibleRect(rect)) { "view has no visible rect: $view" }
        return rect
    }

    private fun findTextView(root: ViewGroup, text: String): TextView? =
        findTextViewWhere(root) { it == text }

    private fun findTextViewWhere(root: ViewGroup, predicate: (String) -> Boolean): TextView? {
        for (i in 0 until root.childCount) {
            val child = root.getChildAt(i)
            if (child is TextView && predicate(child.text.toString())) return child
            if (child is ViewGroup) {
                findTextViewWhere(child, predicate)?.let { return it }
            }
        }
        return null
    }
}
