package edits.block
object O { val x = 1; val y = 2 }
object Main {
  def run: Int = { import O.x; 2 }
  def other: Int = {
    import O.{x, y}
    x
  }
}
