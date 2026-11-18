// expect: 30:30: error: type mismatch: found Int => 5, required Int => Unit
// expect: 34:30: error: type mismatch: found Int => 5, required Int => Unit
// expect: 40:30: error: type mismatch: found Int => 5, required Int => Unit
// expect: 44:38: error: type mismatch: found Int => 5, required Int => Unit
// expect: 48:30: error: type mismatch: found Int => 5, required Int => Unit
// expect: 51:70: error: type mismatch: found Int => Meter, required Int => Unit | Null
// expect: 58:67: error: type mismatch: found Int => Meter, required Int => U
// expect: 63:73: error: type mismatch: found Int => Meter, required Int => r.R
// expect: 68:31: error: type mismatch: found Int => Meter | Unit, required Int => Unit
// expect: 73:16: error: type mismatch: found Int => Meter | Unit, required Int => Unit
// expect: 74:16: error: type mismatch: found Int => Meter | Unit, required Int => Unit
// expect: 75:16: error: type mismatch: found Int => Meter, required Int => Unit
// expect: 76:16: error: type mismatch: found Int => Meter | Unit, required Int => Unit
// A lambda's result that does not conform to its open variable is adapted again only where the variable can be
// defined further to a type the value is dropped to. A variable DECLARED below `Unit` is not such a one: its
// instance is the value's type, which the bound refuses, also for a branch inside a case. Nor is one that
// `Unit | Null`, an opaque subtype of `Unit` seen from outside or an abstract member bounds. The mismatch is told
// at the lambda, its function type against the expected one; a branch not recovered before the one that bounds
// the variable (`FirstBranchUnbounded`) is not recovered by it.
class Cell[+A](val a: A):
  def map[B](f: A => B): Cell[B] = Cell(f(a))
  def flatMap[B](f: A => Cell[B]): Cell[B] = f(a)
  override def toString = s"Cell($a)"
class Meter:
  def tick(): Meter = { println("tick"); this }
val meter = Meter()

object DeclaredBound:
  def f[B <: Unit](g: Int => B): B = g(1)
  def go(): Unit = println(f(_ => 5))

object DeclaredCurried:
  def f[B <: Unit](g: Int => B)(b: B): B = g(1)
  def go(): Unit = println(f(_ => 5)(()))

object DeclaredEvidence:
  trait Show[A] { def name: String }
  given Show[Unit] with { def name = "unit" }
  def f[B <: Unit: Show](g: Int => B): String = summon[Show[B]].name
  def go(): Unit = println(f(_ => 5))

object DeclaredExpected:
  def f[B <: Unit](g: Int => B): B = g(1)
  def go(): Unit = { val u: Unit = f(_ => 5); println(u) }

object DeclaredLater:
  def f[B <: Unit](g: Int => B, b: B): B = g(1)
  def go(): Unit = println(f(_ => 5, ()))

object UnionBound:
  def settle(o: Cell[Int]): Cell[Unit | Null] = o.flatMap(_ => o.map(_ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object OpaqueBound:
  object Hidden:
    opaque type U <: Unit = Unit

  def settle(o: Cell[Int]): Cell[Hidden.U] = o.flatMap(_ => o.map(_ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object MemberBound:
  trait Result { type R <: Unit }
  def settle(r: Result)(o: Cell[Int]): Cell[r.R] = o.flatMap(_ => o.map(_ => meter.tick()))
  def go(): Unit = println("compiled")

object DeclaredThroughCase:
  def mp[B <: Unit](f: Int => B): Cell[B] = Cell(f(1))
  def go(): Unit = println(mp(x => x match { case 1 => if x > 0 then meter.tick() else (); case _ => () }))

object FirstBranchUnbounded:
  def settleUnit[B <: Unit](g: Int => B): B = g(0)
  def go(): Unit =
    settleUnit(x => if x > 0 then meter.tick() else ())
    settleUnit(x => x match { case 1 => meter.tick(); case _ => () })
    settleUnit(x => if x > 0 then meter.tick() else meter.tick())
    settleUnit(x => try meter.tick() catch { case _: Exception => () })


// A handler that is an expression is typed first, against `Throwable => B`: one whose result is no `Unit` is
// refused at the handler.
object HandlerNotUnit:
  def settleUnit[B <: Unit](g: Int => B): B = g(0)
  def count(t: Throwable): Int = 7
  def counter[A](a: A): Throwable => A = _ => a
  def go(): Unit =
    settleUnit(x => try println("body") catch count)
    settleUnit(x => try println("body") catch counter(7))
// expect: 86:47: error: type mismatch: found Throwable => Int, required Throwable => Unit
// expect: 87:55: error: type mismatch: found 7, required Unit
