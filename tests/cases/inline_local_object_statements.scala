// A statement of the body of an object an inline method's body makes reads a private member of the enclosing class
// through its accessor (PrepareInlineable), in the template the pickle writes as in the class's initialization, as a
// field's initializer does. scalac prints the lines of the .expected file.
class Counter(private var cnt: Int):
  private def step: Int = 2
  inline def bump: Int =
    object S:
      cnt += step
      println(cnt)
      val after = cnt * 10
    S.after
@main def run(): Unit =
  val c = Counter(1)
  println(c.bump)
  println(c.bump)
