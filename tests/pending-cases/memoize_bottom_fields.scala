//> using platform jvm
// Pending on memoize-bottom-fields (proposed for the queue): for a val of type Null,
// Nothing or Unit dotty's `Memoize.isErasableBottomField` makes its getter answer
// `null`, throw or `BoxedUnit.UNIT` and `Constructors` drops the field nothing reads
// then, its initialiser run for its effects; the expectation is scalac 3.8.4's.
// Vals of type Null and Unit in an object, a class and a file module, final or not, a var among them: their values,
// then each class's declared fields. Read by reflection, on the JVM.
package bottom
object O:
  val n = null
  final val fn = null
  val u = { println("o unit"); () }
  var vu: Unit = ()
class C:
  val n = null
  val u = ()
val tn = null
val tu = { println("top unit"); () }
@main def run(): Unit =
  println(s"${O.n} ${O.fn} ${O.u} ${new C().n} ${new C().u} $tn $tu")
  for c <- List("bottom.O$", "bottom.C", "bottom.memoize_bottom_fields$package$") do
    println(c + ": " + Class.forName(c).getDeclaredFields.map(_.getName).sorted.mkString(","))
