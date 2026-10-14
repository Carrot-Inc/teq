// An export's stable forwarder of a val has the val's singleton result; the numeric adaptation
// widens that singleton before the conversion (dotty's adaptation of `tree.tpe.widen`).
object Impl:
  val n: Int = 7
object Facade:
  export Impl.n
@main def run(): Unit =
  val x: Long = Facade.n
  println(x)
