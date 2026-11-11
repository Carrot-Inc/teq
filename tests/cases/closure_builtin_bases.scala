trait Thunk extends Function0[Int] with java.io.Serializable:
  def apply(): Int
object Example:
  val thunk: Thunk = new Thunk:
    def apply(): Int = 7
@main def run(): Unit = println(Example.thunk())
