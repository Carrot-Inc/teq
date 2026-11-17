package uma

// Members declared `Unit` whose bodies are values, written to products and read by another module: each is
// evaluated once and its value dropped, through an inline expansion, a lazy value, a given, a variable, a
// class's fields in their order and a generic member's bridge.

object Meter:
  var ticks = 0
  def tick(label: String): Int = { ticks += 1; println(label + ticks); ticks }
  def seen(x: Any): String = x match
    case () => "unit"
    case other => "a value: " + other

object Board:
  inline def expanded: Unit = Meter.tick("inline")
  inline def chosen(b: Boolean): Unit = if b then Meter.tick("branch") else Meter.tick("other")
  def plain: Unit = Meter.tick("plain")

object Kept:
  lazy val once: Unit = Meter.tick("lazy")
  given mark: Unit = Meter.tick("given")
  var slot: Unit = Meter.tick("var")

trait Source[A]:
  def next: A

class Fields extends Source[Unit]:
  val first: Unit = Meter.tick("first")
  val second: Int = Meter.tick("second")
  val third: Unit = Meter.tick("third")
  def next: Unit = Meter.tick("bridge")
