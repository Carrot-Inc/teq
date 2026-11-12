// The thread-safe lazy val's members as dotty's LazyVals writes them: per lazy member a `private volatile Object x$lzyN`
// container, a `private static final VarHandle x$lzyN$lzyHandle` looked up in a `private static <clinit>` (in an
// object's module class too, whose container stays an instance field), a `private Object x$lzyINITN()` and the getter
// of the member's own descriptor and flags; for a class's trait's lazy val, an object member of a class, a given
// alias and an instance's given object the same; the numbers counted per compilation unit in dotty's order, the
// package object's after the file's classes and objects (Desugar.packageDef), a local lazy val's among them. A given
// alias, of `this` or of anything else, keeps its holder here (tests/pending-cases/lazy_given_uncached).
// abi: ref ref$lzy1 ref$lzy1$lzyHandle ref$lzyINIT1 ref$lzy2 ref$lzy2$lzyHandle ref$lzyINIT2 ref$lzy3 ref$lzy3$lzyHandle ref$lzyINIT3 prim prim$lzy1 prim$lzy1$lzyHandle prim$lzyINIT1 t t$lzy1 t$lzy1$lzyHandle t$lzyINIT1 Inner Inner$lzy1 Inner$lzy1$lzyHandle Inner$lzyINIT1 alias alias$lzy1 alias$lzy1$lzyHandle alias$lzyINIT1 obj obj$lzy1 obj$lzy1$lzyHandle obj$lzyINIT1 shared shared$lzy1 shared$lzy1$lzyHandle shared$lzyINIT1 shared$lzy2 shared$lzy2$lzyHandle shared$lzyINIT2 kept kept$lzy1 kept$lzy1$lzyHandle kept$lzyINIT1 $less $less$lzy1 $less$lzy1$lzyHandle $less$lzyINIT1 $greater $greater$lzy2 $greater$lzy2$lzyHandle $greater$lzyINIT2 $less$lzy3 $less$lzy3$lzyHandle $less$lzyINIT3 <clinit>
package lazy_vals

lazy val shared: String = "top-level, numbered after the classes"

class Member:
  lazy val ref: String = "member"
  lazy val prim: Int = 42
  def usesLocal: Int =
    lazy val shared: Int = 1
    shared
  object Inner:
    val v = 1
  given alias: Ordering[Int] = Ordering.Int

trait T:
  lazy val t: String = "trait"

class Impl extends T

object Obj:
  lazy val ref: String = "object"

trait Show:
  def show: String

class Instances:
  given obj: Show with
    def show = "given object"

class Second:
  lazy val ref: String = "second"

class Ctx(val name: String):
  given kept: Ordering[Int] = Ordering.Int

// `<` and `>` share FreshNameCreator's counter: StdNames.str.sanitize makes each prefix `$`, apart from the
// names' encoding.
class Angles:
  lazy val `<` : Int = 1
  lazy val `>` : Int = 2

class AnglesAfter:
  lazy val `<` : Int = 3
