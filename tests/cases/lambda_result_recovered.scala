// teq: --wconf msg=Discarded.non-Unit.value:s
// A lambda's result that does not conform to the open variable it is typed against is adapted again once the
// variable is defined further: the value's type first, then the variable; where the instance is `Unit` the value
// is evaluated and dropped. One object a kind: the comprehension's yield and its written calls, the result's
// places (a binding, a block, a branch, a case, a handler, a guard, a pattern, a nested lambda, a context
// function, a method's name, `Nil`, `None`, a pure value), the bound (an alias of `Unit`, an intersection), the
// depth of the nesting, the variable met twice or late, the shape around it (a class's parameter, a function's
// result, a union, a constructor of a higher kind), an ascription and written type arguments, a view to `Unit`
// that is applied where one is in scope, and a variable declared below `Unit` that an earlier branch already
// bounded from below: an `if`'s, a case's, a `try`'s body, nested and at a block's end.
class Cell[+A](val a: A):
  def map[B](f: A => B): Cell[B] = Cell(f(a))
  def flatMap[B](f: A => Cell[B]): Cell[B] = f(a)
  def withFilter(p: A => Boolean): Cell[A] = { assert(p(a)); this }
  override def toString = s"Cell($a)"
class Meter:
  def tick(): Meter = { println("tick"); this }
val meter = Meter()

object P1:
  def settle(o: Cell[Int]): Cell[Unit] = for { _ <- o; _ <- o } yield meter.tick()
  def go(): Unit = println(settle(Cell(1)))

object P2:
  def settle(o: Cell[Int]): Cell[Unit] = for { _ <- o; _ <- o; _ <- o } yield meter.tick()
  def go(): Unit = println(settle(Cell(1)))

object P3:
  def settle(o: Cell[Int]): Cell[Unit] = for { _ <- o; _ <- o } yield 5
  def go(): Unit = println(settle(Cell(1)))

object P7:
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(_ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object P8:
  def wrap[A](op: Int => Cell[A]): Cell[A] = op(1)
  def settle(o: Cell[Int]): Cell[Unit] = wrap(_ => o.map(_ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object SiteBinding:
  def settle(o: Cell[Int]): Cell[Unit] = for { x <- o; y = x + 1; z <- o } yield meter.tick()
  def go(): Unit = println(settle(Cell(1)))

object SiteBlock:
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(_ => { println("block"); meter.tick() }))
  def go(): Unit = println(settle(Cell(1)))

object SiteContext:
  def take[A](f: Int ?=> A): Cell[A] = Cell(f(using 1))
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => take((x: Int) ?=> meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object SiteEta:
  def step(x: Int): Meter = meter.tick()
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(step))
  def go(): Unit = println(settle(Cell(1)))

object SiteGuard:
  def settle(o: Cell[Int]): Cell[Unit] = for { x <- o; if x > 0; y <- o } yield meter.tick()
  def go(): Unit = println(settle(Cell(1)))

object SiteIf:
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(_ => if true then meter.tick() else meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object SiteMatch:
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(_ => 1 match { case 1 => meter.tick(); case _ => meter.tick() }))
  def go(): Unit = println(settle(Cell(1)))

object SiteNestedLambda:
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(_ => (x: Int) => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object SiteNil:
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(_ => Nil))
  def go(): Unit = println(settle(Cell(1)))

object SiteNone:
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(_ => None))
  def go(): Unit = println(settle(Cell(1)))

object SitePartial:
  def take[A](f: PartialFunction[Int, A]): Cell[A] = Cell(f(1))
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => take { case _ => meter.tick() })
  def go(): Unit = println(settle(Cell(1)))

object SitePattern:
  def settle(o: Cell[Int]): Cell[Unit] = for { (x, y) <- Cell((1, 2)); z <- o } yield meter.tick()
  def go(): Unit = println(settle(Cell(1)))

object SitePure:
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(_ => "pure"))
  def go(): Unit = println(settle(Cell(1)))

object SiteTry:
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => o.map(_ => try meter.tick() finally println("finally")))
  def go(): Unit = println(settle(Cell(1)))

object BoundAlias:
  type U = Unit

  def settle(o: Cell[Int]): Cell[U] = o.flatMap(_ => o.map(_ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object BoundIntersection:
  def settle(o: Cell[Int]): Cell[Unit & Matchable] = o.flatMap(_ => o.map(_ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object NestedUnitDepth:
  def test: Cell[Unit] = for { a <- Cell(1); b <- Cell(1); meter <- Cell(1); d <- Cell(1); e <- Cell(1); f <- Cell(1); g <- Cell(1) } yield 5
  def go(): Unit = println(test)

object NestedNumericDepth:
  def test: Cell[Long] = for { a <- Cell(1); b <- Cell(1); meter <- Cell(1); d <- Cell(1); e <- Cell(1); f <- Cell(1); g <- Cell(1) } yield 5
  def go(): Unit = println(test)

object Twice:
  def settle(o: Cell[Int]): Cell[Cell[Unit]] = o.flatMap(_ => o.map(_ => o.flatMap(_ => o.map(_ => meter.tick()))))
  def go(): Unit = println(settle(Cell(1)))

object RiskLateUnit:
  def f[B](g: Int => B, b: B): Cell[B] = Cell(g(1))
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => f(_ => meter.tick(), ()))
  def go(): Unit = println(settle(Cell(1)))

object StructBox:
  def take[A](f: Int => Cell[A]): Cell[A] = f(1)
  def settle(o: Cell[Int]): Cell[Unit] = take(_ => o.map(_ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object StructFunction:
  def take[A](f: Int => (Int => A)): Cell[A] = Cell(f(1)(2))
  def settle(o: Cell[Int]): Cell[Unit] = o.flatMap(_ => take(_ => _ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object StructUnion:
  def take[A](f: Int => (Cell[A] | String)): Cell[A] = f(1).asInstanceOf[Cell[A]]
  def settle(o: Cell[Int]): Cell[Unit] = take(_ => o.map(_ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object HigherKind:
  def outer[F[_], A](op: Int => F[A]): F[A] = op(1)
  def settle(o: Cell[Int]): Cell[Unit] = outer(_ => o.map(_ => meter.tick()))
  def go(): Unit = println(settle(Cell(1)))

object AscribedRecovery:
  def test: Cell[Unit] = Cell(1).flatMap(_ => Cell(1).map(_ => (identity: Any => Any)))
  def go(): Unit = println(test)

object ExplicitRecovery:
  def test: Cell[Unit] = Cell(1).flatMap(_ => Cell(1).map(_ => (x: Any) => x))
  def go(): Unit = println(test)

object DeclaredView:
  import scala.language.implicitConversions
  given Conversion[Meter, Unit] with
    def apply(meter: Meter): Unit = println("view")
  def f[B <: Unit](g: Int => B): Cell[B] = Cell(g(1))
  def go(): Unit = println(f(_ => meter.tick()))

object ViewUnit:
  import scala.language.implicitConversions
  given Conversion[Meter, Unit] with
    def apply(meter: Meter): Unit = println("view")
  def settle(o: Cell[Int]): Cell[Unit] = o.map(_ => meter.tick())
  def go(): Unit = println(settle(Cell(1)))

object DeclaredAfterUnit:
  def mp[B <: Unit](f: Int => B): Cell[B] = Cell(f(1))
  def go(): Unit = println(mp(x => x match { case 0 => (); case _ => if x > 0 then meter.tick() else () }))

object BranchesInOrder:
  def settleUnit[B <: Unit](g: Int => B): B = g(0)
  def go(): Unit =
    settleUnit(x => if x > 0 then () else meter.tick())
    settleUnit(x => x match { case 1 => (); case _ => meter.tick() })
    settleUnit(x => x match { case 1 => (); case 2 => meter.tick(); case _ => meter.tick() })
    settleUnit(x => { val y = x + 1; if y > 5 then () else meter.tick() })
    settleUnit(x => if x > 0 then () else if x < -5 then meter.tick() else ())
    settleUnit(x => if x > 0 then (if x > 3 then () else meter.tick()) else ())
    settleUnit(x => try () catch { case _: Exception => meter.tick() })
    println("branches done")

@main def run(): Unit =
  println("== p1"); P1.go()
  println("== p2"); P2.go()
  println("== p3"); P3.go()
  println("== p7"); P7.go()
  println("== p8"); P8.go()
  println("== site binding"); SiteBinding.go()
  println("== site block"); SiteBlock.go()
  println("== site context"); SiteContext.go()
  println("== site eta"); SiteEta.go()
  println("== site guard"); SiteGuard.go()
  println("== site if"); SiteIf.go()
  println("== site match"); SiteMatch.go()
  println("== site nested lambda"); SiteNestedLambda.go()
  println("== site nil"); SiteNil.go()
  println("== site none"); SiteNone.go()
  println("== site partial"); SitePartial.go()
  println("== site pattern"); SitePattern.go()
  println("== site pure"); SitePure.go()
  println("== site try"); SiteTry.go()
  println("== bound alias"); BoundAlias.go()
  println("== bound intersection"); BoundIntersection.go()
  println("== nested unit depth"); NestedUnitDepth.go()
  println("== nested numeric depth"); NestedNumericDepth.go()
  println("== twice"); Twice.go()
  println("== risk late unit"); RiskLateUnit.go()
  println("== struct box"); StructBox.go()
  println("== struct function"); StructFunction.go()
  println("== struct union"); StructUnion.go()
  println("== higher kind"); HigherKind.go()
  println("== ascribed recovery"); AscribedRecovery.go()
  println("== explicit recovery"); ExplicitRecovery.go()
  println("== declared view"); DeclaredView.go()
  println("== view unit"); ViewUnit.go()
  println("== declared after unit"); DeclaredAfterUnit.go()
  println("== branches in order"); BranchesInOrder.go()
