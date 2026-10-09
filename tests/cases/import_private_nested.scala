// A private object nested in a class is imported inside the class through a stable alias of
// `this`, as any accessible member is.
class O(val n: Int):
  private object P:
    val hidden = n + 1
    def twice: Int = hidden * 2
  def run(): Int =
    val alias = this
    import alias.P.*
    hidden + twice

@main def run(): Unit = println(new O(42).run())
