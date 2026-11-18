trait ParentT(n: Int) {
  val first = read + n
  def read: Int
}
class Mix extends ParentT(1) {
  val later = 2
  def read: Int = later
}
class LazyRead {
  val first = delayed
  lazy val delayed = later
  val later = 1
}
class Secondary(val p: Int) {
  def this() = { this(1); println(later) }
  val later = 1
}
class TryReturn {
  val first = read()
  def read(): Int = try { return later } catch { case _: Throwable => 0 }
  val later = 1
}
class Statements {
  val first = 1
  println(later)
  val later = 2
  println(first)
}
