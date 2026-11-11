trait Empty():
  val prefix: String = "empty"
trait Middle extends Empty
class Indirect extends Middle
object Singleton extends Middle
trait Supplied(val count: Int)
trait Through extends Supplied
class Direct extends Through with Supplied(7)
class Inherited extends Direct

@main def run(): Unit =
  println(new Indirect().prefix)
  println(Singleton.prefix)
  println(new Direct().count)
  println(new Inherited().count)
