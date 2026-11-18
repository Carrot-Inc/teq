class ByName {
  def run(f: => Int): Int = f
  val a = run(b)
  val b = 10
}
