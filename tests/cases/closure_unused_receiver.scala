trait Deferred extends Function0[Int] with java.io.Serializable:
  def apply(): Int
object Builder:
  val fallback: Deferred = new Deferred:
    def apply(): Int = 0
  val started: Unit = println("init")
  inline def make(inline value: Int, choose: Boolean): Deferred =
    val thunk: Deferred = new Deferred:
      def apply(): Int = value
    if choose then thunk else fallback
@main def run(): Unit =
  println(Builder.make(0, false)())
  val first = Builder.make(7, true)
  var state = 8
  val second = Builder.make(state, true)
  println("made")
  println(first())
  state = 9
  println(second())
