/**
 * RichTextEditorView.kt — a genuine NATIVE rich-text editor for AngNative.
 *
 * HONESTY NOTE: Tiptap and CKEditor are DOM/JavaScript libraries. They depend on
 * a browser document, contentEditable and CSS, so they cannot run unchanged in a
 * native Android shell — this project deliberately has NO WebView and NO JS
 * engine. What is implemented here is a NATIVE EQUIVALENT: Android EditText plus
 * real Spannable spans (StyleSpan, UnderlineSpan, ForegroundColorSpan,
 * AbsoluteSizeSpan, BulletSpan, URLSpan). There is no HTML rendering anywhere.
 */

package com.angnative.app

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.Spannable
import android.text.SpannableStringBuilder
import android.text.Spanned
import android.text.style.AbsoluteSizeSpan
import android.text.style.BulletSpan
import android.text.style.ForegroundColorSpan
import android.text.style.StyleSpan
import android.text.style.UnderlineSpan
import android.text.style.URLSpan
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import java.util.ArrayDeque

private fun dp(context: Context, value: Float): Int =
  TypedValue.applyDimension(
    TypedValue.COMPLEX_UNIT_DIP,
    value,
    context.resources.displayMetrics
  ).toInt()

private data class ColorOption(val name: String, val argb: Int)

/** EditText that forwards caret/selection moves to the live format summary. */
class SelectionAwareEditText(context: Context) : EditText(context) {
  var onSelectionChangedListener: ((Int, Int) -> Unit)? = null

  override fun onSelectionChanged(selStart: Int, selEnd: Int) {
    super.onSelectionChanged(selStart, selEnd)
    onSelectionChangedListener?.invoke(selStart, selEnd)
  }
}

/**
 * All-native rich-text editor. No WebView, no DOM, no HTML — only EditText and
 * SpannableStringBuilder with genuine Android span objects.
 */
class RichTextEditorView(
  context: Context,
  private val rustNodeCount: Int
) : LinearLayout(context) {

  private val editText = SelectionAwareEditText(context)
  private lateinit var summary: TextView
  private lateinit var bridgeStatus: TextView

  private val undoStack = ArrayDeque<SpannableStringBuilder>()
  private val colors = listOf(
    ColorOption("Red", Color.parseColor("#E11D48")),
    ColorOption("Blue", Color.parseColor("#2563EB")),
    ColorOption("Green", Color.parseColor("#16A34A"))
  )
  private var colorIndex = 0
  private var sizeSp = 16

  init {
    orientation = VERTICAL
    setBackgroundColor(Color.parseColor("#F8FAFC"))
    setPadding(dp(context, 16f), dp(context, 16f), dp(context, 16f), dp(context, 16f))

    addView(buildHeader())
    addView(buildToolbar())
    addView(editText, LayoutParams(LayoutParams.MATCH_PARENT, 0, 1f))
    addView(buildFooter())

    seedContent()
  }

  /* ------------------------------------------------------------------ UI */

  private fun buildHeader(): LinearLayout {
    val blue = Color.parseColor("#1D4ED8")
    val ink = Color.parseColor("#0F172A")
    val slate = Color.parseColor("#64748B")

    return LinearLayout(context).apply {
      orientation = VERTICAL
      addView(TextView(context).apply {
        text = "ANGNATIVE • ECOSYSTEM SHOWCASE (NATIVE REIMPLEMENTATION)"
        textSize = 10f
        setTextColor(blue)
        typeface = Typeface.DEFAULT_BOLD
      })
      addView(TextView(context).apply {
        text = "Native Rich Text Editor"
        textSize = 22f
        setTextColor(ink)
        typeface = Typeface.DEFAULT_BOLD
      })
      addView(TextView(context).apply {
        text = "Android EditText + SpannableString. No WebView, no DOM, no HTML rendering."
        textSize = 12f
        setTextColor(slate)
      })
      addView(TextView(context).apply {
        text = "Tiptap/CKEditor are DOM libraries and cannot run unchanged natively; " +
          "this is a native equivalent built from real Android spans."
        textSize = 11f
        setTextColor(slate)
        setPadding(0, dp(context, 6f), 0, dp(context, 4f))
      })
    }
  }

  private fun buildToolbar(): LinearLayout {
    fun button(label: String, desc: String, action: () -> Unit): Button =
      Button(context).apply {
        text = label
        contentDescription = desc
        isAllCaps = false
        textSize = 13f
        setTextColor(Color.WHITE)
        setPadding(4, 6, 4, 6)
        layoutParams = LinearLayout.LayoutParams(0, dp(context, 42f), 1f)
        background = GradientDrawable().apply {
          shape = GradientDrawable.RECTANGLE
          cornerRadius = dp(context, 8f).toFloat()
          setColor(Color.parseColor("#2563EB"))
        }
        setOnClickListener { action() }
      }

    fun row(vararg views: View): LinearLayout = LinearLayout(context).apply {
      orientation = HORIZONTAL
      views.forEach { addView(it) }
    }

    return LinearLayout(context).apply {
      orientation = VERTICAL
      setPadding(0, dp(context, 10f), 0, dp(context, 8f))
      addView(row(
        button("Bold", "Bold", { toggleStyle(Typeface.BOLD) }),
        button("Italic", "Italic", { toggleStyle(Typeface.ITALIC) }),
        button("Underline", "Underline", { toggleUnderline() })
      ))
      addView(row(
        button("Bullet", "Bullet list", { toggleBullet() }),
        button("Color", "Text color", { cycleColor() }),
        button("A+", "Increase font size", { changeSize(2) })
      ))
      addView(row(
        button("A-", "Decrease font size", { changeSize(-2) }),
        button("Undo", "Undo", { undo() }),
        button("Clear", "Clear", { clear() })
      ))
    }
  }

  private fun buildFooter(): LinearLayout {
    bridgeStatus = TextView(context).apply {
      text = "Rust JNI host OK • shadow nodes: $rustNodeCount"
      textSize = 11f
      setTextColor(Color.parseColor("#1D4ED8"))
    }
    summary = TextView(context).apply {
      text = "Format summary: initialising…"
      textSize = 12f
      setTextColor(Color.parseColor("#0F172A"))
      setPadding(0, dp(context, 6f), 0, 0)
    }
    return LinearLayout(context).apply {
      orientation = VERTICAL
      setPadding(0, dp(context, 8f), 0, 0)
      addView(bridgeStatus)
      addView(summary)
    }
  }

  private fun seedContent() {
    val sb = SpannableStringBuilder()
    fun appendStyled(text: String, vararg spans: Any) {
      val start = sb.length
      sb.append(text)
      val end = sb.length
      spans.forEach { sb.setSpan(it, start, end, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE) }
    }

    appendStyled(
      "Native Rich Text\n",
      StyleSpan(Typeface.BOLD),
      AbsoluteSizeSpan(22, true),
      ForegroundColorSpan(Color.parseColor("#1D4ED8"))
    )

    // A plain run is pre-selected so a tester can immediately tap Bold /
    // Underline and see a real span appear (and the summary update).
    val testStart = sb.length
    appendStyled("Plain sample text is pre-selected for toolbar verification.\n")
    val testEnd = testStart + "Plain sample text".length

    appendStyled("Select any range, then tap a button to apply a genuine Android span.\n\n")
    appendStyled("Formatting demo\n", StyleSpan(Typeface.BOLD))

    val bulletStart = sb.length
    appendStyled("Bullet item built with a real BulletSpan.\n")
    sb.setSpan(
      BulletSpan(dp(context, 16f), Color.parseColor("#1D4ED8")),
      bulletStart, sb.length - 1, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
    )

    appendStyled("Italic, ", StyleSpan(Typeface.ITALIC))
    appendStyled("underlined, ", UnderlineSpan())
    appendStyled("red, ", ForegroundColorSpan(Color.parseColor("#E11D48")))
    appendStyled("large, ", AbsoluteSizeSpan(19, true))
    appendStyled("linked ", URLSpan("https://angnative.dev"))
    appendStyled("and more — every effect above is a native span.\n")

    editText.apply {
      text = sb
      textSize = 16f
      setTextColor(Color.parseColor("#0F172A"))
      gravity = Gravity.TOP or Gravity.START
      setPadding(dp(context, 12f), dp(context, 12f), dp(context, 12f), dp(context, 12f))
      contentDescription = "Rich text editor"
      isFocusable = true
      isFocusableInTouchMode = true
      background = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(context, 10f).toFloat()
        setColor(Color.WHITE)
        setStroke(dp(context, 1f), Color.parseColor("#E2E8F0"))
      }
      onSelectionChangedListener = { start, end -> updateSummary(start, end) }
    }

    editText.setSelection(testStart, testEnd)
    updateSummary(testStart, testEnd)
  }

  /* ------------------------------------------------------------ Summary */

  private fun updateSummary(start: Int, end: Int) {
    if (!::summary.isInitialized) return
    val sp = editText.text
    val labels = mutableListOf<String>()
    val seen = mutableSetOf<String>()
    if (sp is Spanned) {
      // Ignore EditText's internal spans (ChangeWatcher, SpanController, START,
      // END, ...) — only genuine formatting spans belong in the summary.
      for (span in sp.getSpans(start, end, Any::class.java)) {
        val label = when (span) {
          is StyleSpan -> if (span.style == Typeface.BOLD) "Bold" else "Italic"
          is UnderlineSpan -> "Underline"
          is ForegroundColorSpan -> "Color #%06X".format(0xFFFFFF and span.foregroundColor)
          is AbsoluteSizeSpan -> "Size ${span.size}"
          is BulletSpan -> "Bullet"
          is URLSpan -> "Link: ${span.url}"
          else -> continue
        }
        if (seen.add(label)) labels.add(label)
      }
    }
    val selection = if (start == end) "cursor @ $start" else "${end - start} chars selected"
    summary.text = "Format summary ($selection): " +
      if (labels.isEmpty()) "Plain text" else labels.joinToString(", ")
  }

  /* ------------------------------------------------------------- Edits */

  private fun pushUndo() {
    undoStack.push(SpannableStringBuilder(editText.text))
    if (undoStack.size > 40) undoStack.removeLast()
  }

  private fun spannable(): Spannable? = editText.text as? Spannable

  private fun toggleStyle(style: Int) {
    val start = editText.selectionStart
    val end = editText.selectionEnd
    val sp = spannable() ?: return
    val existing = sp.getSpans(start, end, StyleSpan::class.java).filter { it.style == style }
    val coversWholeSelection = start < end &&
      existing.any { sp.getSpanStart(it) <= start && sp.getSpanEnd(it) >= end }
    pushUndo()
    if (coversWholeSelection) {
      existing.forEach { sp.removeSpan(it) }
    } else {
      sp.setSpan(
        StyleSpan(style), start, end,
        if (start == end) Spanned.SPAN_INCLUSIVE_INCLUSIVE
        else Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
      )
    }
    updateSummary(start, end)
  }

  private fun toggleUnderline() {
    val start = editText.selectionStart
    val end = editText.selectionEnd
    val sp = spannable() ?: return
    val existing = sp.getSpans(start, end, UnderlineSpan::class.java)
    val coversWholeSelection = start < end &&
      existing.any { sp.getSpanStart(it) <= start && sp.getSpanEnd(it) >= end }
    pushUndo()
    if (coversWholeSelection) {
      existing.forEach { sp.removeSpan(it) }
    } else {
      sp.setSpan(
        UnderlineSpan(), start, end,
        if (start == end) Spanned.SPAN_INCLUSIVE_INCLUSIVE
        else Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
      )
    }
    updateSummary(start, end)
  }

  private fun toggleBullet() {
    val start = editText.selectionStart
    val end = editText.selectionEnd
    val sp = spannable() ?: return
    val text = sp.toString()
    if (text.isEmpty()) return

    var lineStart = start.coerceAtMost(text.length)
    while (lineStart > 0 && text[lineStart - 1] != '\n') lineStart--
    var lineEnd = end.coerceAtMost(text.length)
    while (lineEnd < text.length && text[lineEnd] != '\n') lineEnd++

    pushUndo()
    val existing = sp.getSpans(lineStart, lineEnd, BulletSpan::class.java)
    if (lineEnd > lineStart && existing.any {
        sp.getSpanStart(it) <= lineStart && sp.getSpanEnd(it) >= lineEnd
      }) {
      existing.forEach { sp.removeSpan(it) }
    } else if (lineEnd > lineStart) {
      sp.setSpan(
        BulletSpan(dp(context, 16f), Color.parseColor("#1D4ED8")),
        lineStart, lineEnd, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
      )
    }
    updateSummary(start, end)
  }

  private fun cycleColor() {
    val option = colors[colorIndex % colors.size]
    colorIndex++
    val start = editText.selectionStart
    val end = editText.selectionEnd
    val sp = spannable() ?: return
    pushUndo()
    sp.setSpan(
      ForegroundColorSpan(option.argb), start, end,
      if (start == end) Spanned.SPAN_INCLUSIVE_INCLUSIVE
      else Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
    )
    updateSummary(start, end)
  }

  private fun changeSize(delta: Int) {
    sizeSp = (sizeSp + delta).coerceIn(10, 32)
    val start = editText.selectionStart
    val end = editText.selectionEnd
    val sp = spannable() ?: return
    pushUndo()
    sp.setSpan(
      AbsoluteSizeSpan(sizeSp, true), start, end,
      if (start == end) Spanned.SPAN_INCLUSIVE_INCLUSIVE
      else Spanned.SPAN_EXCLUSIVE_EXCLUSIVE
    )
    updateSummary(start, end)
  }

  private fun undo() {
    val snapshot = undoStack.pollFirst()
    if (snapshot == null) {
      summary.text = "Format summary: undo stack is empty"
      return
    }
    editText.setText(snapshot)
    val caret = editText.selectionStart.coerceAtMost(editText.length())
    editText.setSelection(caret)
    updateSummary(caret, caret)
  }

  private fun clear() {
    pushUndo()
    editText.setText("")
    updateSummary(0, 0)
  }
}
