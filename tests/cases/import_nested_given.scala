// A given imported from an object nested in a class (`import a.R.given`) is read on the
// imported value, not on a lexical enclosing instance.
trait Show:
  def n: Int

class O(val i: Int):
  object R:
    given Show = new Show:
      def n: Int = i + 100

@main def run(): Unit =
  val a = new O(42)
  val b = new O(1)
  locally:
    import a.R.given
    println(summon[Show].n)
  locally:
    import b.R.given
    println(summon[Show].n)
