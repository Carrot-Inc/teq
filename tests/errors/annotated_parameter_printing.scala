import scala.annotation.internal.RuntimeChecked
object AnnotatedParameterPrinting:
  extension (x: Int)
    inline def f(inline y: Int): Int = x + y
  def explicit(x: Int @RuntimeChecked): Int = x
  val checked = 1.runtimeChecked
  val a = checked.f()
  val b = explicit()
object ExportAnnotatedParameterPrinting:
  export AnnotatedParameterPrinting.f
  val c = f(1.runtimeChecked)()

// PlainPrinter.scala:324-337 and Definitions.scala:1152-1154 at 3.8.4:
// InlineParam is silent; RuntimeChecked remains visible.
// expect: 7:20: error: missing argument for parameter y of method f in object AnnotatedParameterPrinting: (y: Int): Int
// expect: 8:19: error: missing argument for parameter x of method explicit in object AnnotatedParameterPrinting: (x: Int @RuntimeChecked): Int
// expect: 11:30: error: missing argument for parameter y of method f in object ExportAnnotatedParameterPrinting: (y: Int): Int
// expect: 3 errors found
