// The library side of tests/classpath/js/bound_argument_over_scalac, compiled by scalac there and to lean
// products in tests/workers.sh: objects whose loading shows, the effect a default of a parent trait's
// parameter, which the object's own class does not write. One holds an extension, one an export, one
// neither, one is called on, one's load is the body of its base's secondary constructor, two are the
// receivers of a right-associative operator, and two are made with an instance of the class their parent is
// nested in, whose path their load evaluates. A pickle holds an extension, an export's forwarder and a
// constructor as methods, so a program compiled against these classes sees nine objects without initialisation.
package boundlib

object Helpers { def helper = 2 }
trait WithExtension(x: Int = { println("  Tagged loaded"); 1 })
object Tagged extends WithExtension { extension (k: Int) def twice: Int = k * 2 }
trait WithExport(x: Int = { println("  Shared loaded"); 1 })
object Shared extends WithExport { export Helpers.helper }
trait WithNeither(x: Int = { println("  Bare loaded"); 1 })
object Bare extends WithNeither
trait WithMethod(x: Int = { println("  Host loaded"); 1 })
object Host extends WithMethod { def take(a: Any, b: Any): Unit = println("  take called") }
class WithSecondary { def this(x: Int) = { this(); println("  Late loaded") } }
object Late extends WithSecondary(1)
trait WithOperator(x: Int = { println("  Cons loaded"); 1 })
object Cons extends WithOperator { def ::(x: Any): Unit = println("  :: called") }
class WithSecondaryOperator { def this(x: Int) = { this(); println("  LateCons loaded") } }
object LateCons extends WithSecondaryOperator(1) { def ::(x: Any): Unit = println("  :: called") }
class WithMember { class Member { def ::(x: Any): Unit = println("  :: called") } }
object Keeper { println("  Keeper loaded"); val made = new WithMember }
object Enclosed extends Keeper.made.Member
object ConsKeeper { println("  ConsKeeper loaded"); val made = new WithMember }
object EnclosedCons extends ConsKeeper.made.Member
def effect(tag: String): Int = { println("  effect " + tag); 1 }
def pair(a: Any, b: Any): Unit = println("  pair called")
