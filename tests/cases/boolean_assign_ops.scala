// `b ||= c` and `b &&= c` on a Boolean var: `b = b || c` and `b = b && c`, the right operand
// evaluated only where the operator needs it (munit's `TagsFilter` is written so).
@main def run(): Unit =
  var calls = 0
  def check(v: Boolean): Boolean = { calls += 1; v }
  var a = false
  a ||= check(true)
  a ||= check(false)
  var b = true
  b &&= check(false)
  b &&= check(true)
  println(s"$a $b $calls")
