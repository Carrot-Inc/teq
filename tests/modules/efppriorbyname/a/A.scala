object Prior {
  def run(): Int = {
    var n = 0
    class C { def m(a: => Int)(b: Int = a): Int = a + b }
    new C().m({ n += 1; n })() * 10 + n
  }
}
