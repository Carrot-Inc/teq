// The field behind A's accessor `name` (`$name` the runtime's, so `$$name`) beside a subclass's val `$$name`
// (`Constructors.checkNoFieldClashes`).
trait Named:
  def name: String
class A(val name: String) extends Named
class B(val $$name: String) extends A("first")
@main def run(): Unit =
  val b = B("second")
  println(b.name)
  println(b.$$name)
