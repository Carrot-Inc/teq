// `import o.R.*` and `import o.R.x` through a value holding an object nested in a class: the
// members are read from that value's object.
class O(val n: Int):
  object R:
    val x = n * 2
    def show(y: Int): String = s"R($n):$y"
    type T = Int

@main def run(): Unit =
  val a = O(1)
  val b = O(2)
  locally:
    import a.R.*
    println(show(x))
  locally:
    import b.R.{x, show}
    val t: b.R.T = x
    println(show(t))
