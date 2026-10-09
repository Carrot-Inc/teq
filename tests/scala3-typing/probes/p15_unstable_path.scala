object T:
  var v = 1
  val t: v.type = v
  def f(c: => Boolean): c.type = ???
  def g(x: Int) = { var q = x; val r: q.type = q; r }
@main def run(): Unit = println(T.t)
