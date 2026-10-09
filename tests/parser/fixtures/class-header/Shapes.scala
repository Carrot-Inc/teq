// A class header broken in its parameters: the class keeps its name, its other parameters, its
// parents and its body, and constructs from any arguments.
trait Base:
  def m: Int = 1
class C(val x: Int, y: ) extends Base:
  def n = x
object Shapes:
  val bad: String = 1
