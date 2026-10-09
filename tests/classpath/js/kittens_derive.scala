// jars: scala-library cats-kernel-sjs cats-core-sjs alleycats-core shapeless3-deriving kittens
// kittens' derivation from its jar: `derives Eq, Show, Order, Hash, Monoid, Semigroup` on case
// classes, sealed traits and enums with nested and recursive types, `Functor`, `Foldable` and
// `Traverse` for a container, and `cats.derived.semiauto`, through shapeless3's inline
// `K0.ProductInstances` and `summonInline` on the lean std.
//> using dep org.typelevel::cats-core:2.13.0
//> using dep org.typelevel::kittens:3.5.0
import cats.*
import cats.derived.*
import cats.syntax.all.*

case class Slot(index: Int, label: String) derives Eq, Show, Order, Hash
case class Money(cents: Long, tags: List[String]) derives Eq, Show, Monoid, Semigroup
case class Wrapper(slot: Slot, money: Money, note: Option[String], scores: Map[String, Int]) derives Eq, Show
enum Tone derives Eq, Show, Order:
  case Low, High
enum Shape derives Eq, Show:
  case Circle(r: Int)
  case Rect(w: Int, h: Int)
  case Dot
sealed trait Expr derives Eq, Show
case class Num(n: Int) extends Expr
case class Add(l: Expr, r: Expr) extends Expr
case class Neg(e: Expr) extends Expr
case class Tree[A](value: A, children: List[Tree[A]]) derives Functor, Foldable, Traverse
case class Pair[A](first: A, second: A) derives Functor, Foldable, Traverse, Eq, Show
case class Empty() derives Eq, Show, Monoid
case object Unit1 derives Eq, Show
case class Stats(count: Int, total: Long, max: Option[Int], names: Set[String]) derives Monoid, Eq, Show

case class Manual(a: Int, b: String)
object Manual:
  given Eq[Manual] = semiauto.eq
  given Show[Manual] = semiauto.show
  given Order[Manual] = semiauto.order
  given Hash[Manual] = semiauto.hash
case class Gen[A](a: A, n: Int)
object Gen:
  given [A: Eq]: Eq[Gen[A]] = semiauto.eq
  given [A: Show]: Show[Gen[A]] = semiauto.show
  given Functor[Gen] = semiauto.functor
  given Traverse[Gen] = semiauto.traverse

object Main:
  def p(xs: Any*): Unit = println(xs.mkString(" "))
  def main(args: Array[String]): Unit =
    val s = Slot(1, "a")
    p((s === Slot(1, "a")), (s =!= Slot(2, "a")), s.show, Order[Slot].compare(s, Slot(1, "b")), (s < Slot(0, "z")), (Hash[Slot].hash(s) == Hash[Slot].hash(Slot(1, "a"))))
    p((Money(1, List("x")) |+| Money(2, List("y"))), Monoid[Money].empty, List(Money(5, Nil), Money(6, List("t"))).combineAll, Money(3, Nil).show, (Money(1, Nil) === Money(1, Nil)))
    val w = Wrapper(s, Money(7, List("q")), Some("n"), Map("k" -> 1))
    p(w.show, (w === w), (w === w.copy(note = None)), (w === w.copy(scores = Map())))
    p((Tone.Low: Tone).show, (Tone.High: Tone).show, ((Tone.Low: Tone) === Tone.Low), ((Tone.Low: Tone) =!= Tone.High), Order[Tone].compare(Tone.Low, Tone.High), List[Tone](Tone.High, Tone.Low).sorted(using Order[Tone].toOrdering))
    p((Shape.Circle(1): Shape).show, (Shape.Rect(2, 3): Shape).show, (Shape.Dot: Shape).show, ((Shape.Circle(1): Shape) === Shape.Circle(1)), ((Shape.Circle(1): Shape) === Shape.Rect(1, 1)), ((Shape.Dot: Shape) === Shape.Dot))
    val e: Expr = Add(Num(1), Neg(Add(Num(2), Num(3))))
    p(e.show, (e === e), (e === Num(1)), ((Num(1): Expr) === Num(1)), List[Expr](Num(1), Neg(Num(1))).show)
    val t = Tree(1, List(Tree(2, Nil), Tree(3, List(Tree(4, Nil)))))
    p(t.map(_ * 10), t.foldLeft(0)(_ + _), t.toList, t.traverse(i => Option(i).filter(_ > 0)).map(_.toList), t.traverse(i => if i > 3 then None else Some(i)), t.size, t.foldMap(_.toString))
    p(Pair(1, 2).map(_ + 1), Pair(1, 2).toList, Pair("a", "b").traverse(s => Option(s.length)), (Pair(1, 2) === Pair(1, 2)), Pair(1, 2).show, Pair(List(1), List(2, 3)).sequence, Pair(3, 4).foldRight(Eval.now(0))((a, b) => b.map(_ + a)).value)
    p(Empty().show, (Empty() === Empty()), Monoid[Empty].empty, Unit1.show, (Unit1 === Unit1))
    p((Stats(1, 10L, Some(3), Set("a")) |+| Stats(2, 5L, Some(5), Set("b"))), Monoid[Stats].empty, Stats(1, 1L, None, Set()).show, (Stats(1, 1L, None, Set()) === Monoid[Stats].empty))
    p((Manual(1, "a") === Manual(1, "a")), Manual(2, "b").show, Order[Manual].compare(Manual(1, "a"), Manual(1, "b")), (Hash[Manual].hash(Manual(1, "a")) == Hash[Manual].hash(Manual(1, "a"))))
    p((Gen(1, 2) === Gen(1, 2)), Gen("s", 3).show, Gen(1, 2).map(_ + 1), Gen(Option(1), 2).sequence, Gen(2, 0).traverse(i => List(i, i * 2)))
    p(List(Slot(2, "b"), Slot(1, "z"), Slot(1, "a")).sorted(using Order[Slot].toOrdering), List(w, w.copy(slot = Slot(0, "a"))).map(_.slot).show, Option(s).show, Map(1 -> s).show, (Some(s): Option[Slot]).map(_.show))
    p(Eq[List[Slot]].eqv(List(s), List(Slot(1, "a"))), Show[Option[Expr]].show(Some(Num(4))), Show[(Slot, Tone)].show((s, Tone.Low)), Order[Option[Slot]].compare(None, Some(s)))
