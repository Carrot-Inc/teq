package cta

// A case class whose companion holds vals: its `apply` runs the companion's initialiser first.
final case class Cursor(at: Int, id: String)
object Cursor:
  private val tag: Int = 1
  val first: Cursor = Cursor(tag, "first")
