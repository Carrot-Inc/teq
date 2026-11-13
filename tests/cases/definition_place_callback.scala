// A block's inline method whose check first needs a later method of the block, its result inferred, that calls the
// inline method back. The method is typed where it is defined, outside the check; the call back is kept for the
// inlining phase, as a member's is, and expanded once the method's result is known.

// Expects "got x": the call back stands in a branch the run does not take. scalac prints got x.
object NotTaken:
  object Test:
    def run(flag: Boolean): String =
      inline def shown: String = "got " + later
      def later = if flag then "x" else shown
      shown
    def main(args: Array[String]): Unit = println(run(true))

// Expects "got got got x": the run takes the call back twice, over a variable of the block. scalac prints
// got got got x.
object Taken:
  object Test:
    def run(count: Int): String =
      var left = count
      inline def shown: String = "got " + later
      def later = { left -= 1; if left <= 0 then "x" else shown }
      shown
    def main(args: Array[String]): Unit = println(run(3))

// Expects "got got x": both methods take a parameter. scalac prints got got x.
object WithParameters:
  object Test:
    def run(count: Int): String =
      inline def shown(k: Int): String = "got " + later(k - 1)
      def later(k: Int) = if k <= 0 then "x" else shown(k)
      shown(count)
    def main(args: Array[String]): Unit = println(run(2))

// Expects "got got xm": the call back goes through a third method of the block, its result inferred too. scalac
// prints got got xm.
object ThroughAThird:
  object Test:
    def run(count: Int): String =
      inline def shown(k: Int): String = "got " + later(k - 1)
      def later(k: Int) = if k <= 0 then "x" else mid(k)
      def mid(k: Int) = shown(k) + "m"
      shown(count)
    def main(args: Array[String]): Unit = println(run(2))

// Expects "got got x": the call back stands in a function value of the method's body. scalac prints got got x.
object InAFunction:
  object Test:
    def run(count: Int): String =
      inline def shown(k: Int): String = "got " + later(k - 1)
      def later(k: Int) = { val f = (j: Int) => if j <= 0 then "x" else shown(j); f(k) }
      shown(count)
    def main(args: Array[String]): Unit = println(run(2))

@main def run(): Unit =
  NotTaken.Test.main(Array())
  Taken.Test.main(Array())
  WithParameters.Test.main(Array())
  ThroughAThird.Test.main(Array())
  InAFunction.Test.main(Array())
