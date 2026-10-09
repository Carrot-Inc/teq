// `import o.R.S.*` through two objects nested in a class reads `S` on `o.R`, which the import
// reads on `o`: the qualifier is kept through every segment.
class O(val n: Int):
  object R:
    object S:
      val x = n
      def twice: Int = x * 2

@main def run(): Unit =
  val o = new O(42)
  println(o.R.S.x)
  import o.R.S.*
  println(x + twice)
  locally:
    import o.R.S.x
    println(x)
