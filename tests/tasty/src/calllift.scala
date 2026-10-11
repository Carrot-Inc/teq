package fix.calllift

// What a call's plan lifts (`TypedApply.liftFun`, `Lifter.liftArgs`) as `TreeInfo`'s purity
// decides it: a lazy val's read and a block of an immutable val (`statPurity`, capped at `Pure`)
// are bound once, a path stays in place, and an export forwarder read as an `Ident` is a stable
// member, a pure path.
object LiftShapes:
  def touch(): Int = 1
  def sum(a: Int, b: Int = 2): Int = a + b
  object Defaults:
    val unit = touch()

object LiftExports:
  export LiftShapes.*

object CallLift:
  def f(a: Int)(b: Int = 1): Int = a + b
  def g(a: Int, b: Int): Int = a + b
  def lazyRead(): Int =
    lazy val id = g(1, 2)
    f(id)()
  def pathRead(): Int =
    val path = 2
    f(path)()
  def valBlock(): Int =
    val path = 2
    f({ val x = path; x })()
  def varBlock(): Int =
    val path = 2
    g(b = touched(), a = { var q = path; q })
  def touched(): Int = 3
  def exported(): Int =
    import LiftExports.*
    sum(b = Defaults.unit, a = 1)
