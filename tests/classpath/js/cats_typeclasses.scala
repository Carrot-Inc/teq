// jars: scala-library cats-kernel-sjs cats-core-sjs
// User instances of cats' type classes for a user type: Functor, Applicative, Monad, Traverse,
// Foldable, Invariant, Parallel, Show, Eq, Semigroup and Monoid, with the syntax and the
// derived methods of cats' traits (map2, replicateA, foldMap, traverse_, ...).
//> using dep org.typelevel::cats-core:2.13.0
import cats.*
import cats.data.{NonEmptyList, Validated}
import cats.syntax.all.*

sealed trait Tree[+A]
case class Leaf[A](value: A) extends Tree[A]
case class Node[A](left: Tree[A], right: Tree[A]) extends Tree[A]

object Tree:
  given treeInstances: (Traverse[Tree] & Monad[Tree]) = new Traverse[Tree] with Monad[Tree]:
    override def map[A, B](fa: Tree[A])(f: A => B): Tree[B] = fa match
      case Leaf(a) => Leaf(f(a))
      case Node(l, r) => Node(map(l)(f), map(r)(f))
    def traverse[G[_]: Applicative, A, B](fa: Tree[A])(f: A => G[B]): G[Tree[B]] = fa match
      case Leaf(a) => f(a).map(Leaf(_))
      case Node(l, r) => (traverse(l)(f), traverse(r)(f)).mapN(Node(_, _))
    def foldLeft[A, B](fa: Tree[A], b: B)(f: (B, A) => B): B = fa match
      case Leaf(a) => f(b, a)
      case Node(l, r) => foldLeft(r, foldLeft(l, b)(f))(f)
    def foldRight[A, B](fa: Tree[A], lb: Eval[B])(f: (A, Eval[B]) => Eval[B]): Eval[B] = fa match
      case Leaf(a) => f(a, lb)
      case Node(l, r) => foldRight(l, Eval.defer(foldRight(r, lb)(f)))(f)
    def pure[A](a: A): Tree[A] = Leaf(a)
    def flatMap[A, B](fa: Tree[A])(f: A => Tree[B]): Tree[B] = fa match
      case Leaf(a) => f(a)
      case Node(l, r) => Node(flatMap(l)(f), flatMap(r)(f))
    def tailRecM[A, B](a: A)(f: A => Tree[Either[A, B]]): Tree[B] = flatMap(f(a)) {
      case Left(a1) => tailRecM(a1)(f)
      case Right(b) => Leaf(b)
    }
  def showTree[A](t: Tree[A])(using Show[A]): String = t match
    case Leaf(a) => s"Leaf(${(a: A).show})"
    case Node(l, r) => s"Node(${showTree[A](l)}, ${showTree[A](r)})"
  def eqTree[A](x: Tree[A], y: Tree[A])(using Eq[A]): Boolean = (x, y) match
    case (Leaf(a), Leaf(b)) => (a: A) === (b: A)
    case (Node(l1, r1), Node(l2, r2)) => eqTree[A](l1, l2) && eqTree[A](r1, r2)
    case _ => false
  given [A: Show]: Show[Tree[A]] = Show.show[Tree[A]](showTree(_))
  given [A: Eq]: Eq[Tree[A]] = Eq.instance[Tree[A]](eqTree(_, _))
  given [A: Semigroup]: Semigroup[Tree[A]] = Semigroup.instance((x, y) => Node(x, y))

final case class Box[A](value: A)
object Box:
  given Functor[Box] with
    def map[A, B](fa: Box[A])(f: A => B): Box[B] = Box(f(fa.value))
  given Applicative[Box] with
    def pure[A](a: A): Box[A] = Box(a)
    def ap[A, B](ff: Box[A => B])(fa: Box[A]): Box[B] = Box(ff.value(fa.value))
  given Foldable[Box] with
    def foldLeft[A, B](fa: Box[A], b: B)(f: (B, A) => B): B = f(b, fa.value)
    def foldRight[A, B](fa: Box[A], lb: Eval[B])(f: (A, Eval[B]) => Eval[B]): Eval[B] = f(fa.value, lb)
  given Invariant[Box] = Invariant[Box]
  given [A: Monoid]: Monoid[Box[A]] = Monoid.instance(Box(Monoid[A].empty), (x, y) => Box(x.value |+| y.value))
  given [A: Show]: Show[Box[A]] = Show.show(b => s"Box(${b.value.show})")

final case class Par[A](run: () => A)
object Par:
  given Applicative[Par] with
    def pure[A](a: A): Par[A] = Par(() => a)
    def ap[A, B](ff: Par[A => B])(fa: Par[A]): Par[B] = Par(() => ff.run()(fa.run()))
  given Parallel.Aux[Box, Par] = new Parallel[Box]:
    type F[A] = Par[A]
    def applicative: Applicative[Par] = summon
    def monad: Monad[Box] = new Monad[Box]:
      def pure[A](a: A): Box[A] = Box(a)
      def flatMap[A, B](fa: Box[A])(f: A => Box[B]): Box[B] = f(fa.value)
      def tailRecM[A, B](a: A)(f: A => Box[Either[A, B]]): Box[B] = f(a).value match
        case Left(a1) => tailRecM(a1)(f)
        case Right(b) => Box(b)
    def sequential: Par ~> Box = new (Par ~> Box) { def apply[A](fa: Par[A]): Box[A] = Box(fa.run()) }
    def parallel: Box ~> Par = new (Box ~> Par) { def apply[A](fa: Box[A]): Par[A] = Par(() => fa.value) }

object Main:
  import Par.given
  def p(xs: Any*): Unit = println(xs.mkString(" "))
  def main(args: Array[String]): Unit =
    val t: Tree[Int] = Node(Leaf(1), Node(Leaf(2), Leaf(3)))
    p(t.show, t.map(_ * 2).show, (t === t), (t === Leaf(1)))
    p(t.foldLeft(0)(_ + _), t.toList, t.foldMap(_.toString), t.size, t.exists(_ > 2), t.find(_ == 2), t.isEmpty)
    p(t.traverse(i => Option(i).filter(_ > 0)).map(_.show), t.traverse(i => if i > 2 then None else Some(i)), t.traverse(i => List(i, i + 10)).size)
    p(t.flatMap(i => Node(Leaf(i), Leaf(-i))).show, (t |+| Leaf(9)).show, Traverse[Tree].traverse[Option, Option[Int], Int](Node(Leaf(Option(1)), Leaf(Option(2))))(identity).map(_.show))
    p(t.traverse_(i => Either.cond(i < 5, i, "big")), t.foldRight(Eval.now(""))((a, b) => b.map(_ + a)).value, t.reduceLeftOption(_ + _), t.zipWithIndex.show)
    p(t.mapWithIndex((a, i) => a * 100 + i).show, t.void.show, t.as("x").show, t.fproduct(_ * 2).toList, t.mapAccumulate(0)((s, a) => (s + a, s))._1, Monad[Tree].replicateA(2, Leaf(1)).show)
    p(Traverse[Tree].compose[Option].map(Leaf(Option(1)))(_ + 1).show, Functor[Tree].lift[Int, Int](_ + 1)(t).show, Foldable[Tree].foldK(Node(Leaf(List(1)), Leaf(List(2)))), Monad[Tree].tailRecM(0)(i => Leaf(if i < 3 then Left(i + 1) else Right(i))).show)
    val b = Box(3)
    p(b.map(_ + 1), (b, Box("a")).mapN((i, s) => s * i), b.foldLeft(1)(_ * _), b.toList, b.show, (b |+| Box(4)), Applicative[Box].pure(1), Applicative[Box].unit)
    p(b.product(Box(true)), b.replicateA(2), Apply[Box].map2(b, b)(_ + _), b.fproduct(_ + 1), b.as(9), Invariant[Box].imap(b)(_ + 1)(_ - 1), b.whenA(false), b.unlessA(false))
    p(List(1, 2, 3).parTraverse(i => Box(i * 2)), (Box(1), Box(2)).parMapN(_ + _), List(Box(1), Box(2)).parSequence, (Box(1), Box(2), Box(3)).parTupled, Parallel[Box].parallel(Box(5)).run())
    p(Functor[Box].widen[Int, Any](b), Foldable[Box].combineAll(Box(4)), Foldable[Box].foldMap(b)(_.toString), Foldable[Box].reduceLeftOption(b)(_ + _), Foldable[Box].get(b)(0), Foldable[Box].toIterable(b).toList)
    p(Monoid[Box[Int]].empty, Monoid[Box[String]].combineAll(List(Box("a"), Box("b"))), Box(Option(1)).show, Show[Box[Box[Int]]].show(Box(Box(1))), Semigroup[Tree[Int]].combineN(Leaf(1), 3).show)
    p(NonEmptyList.of(Box(1), Box(2)).reduce, Validated.valid[String, Box[Int]](b).map(_.value), Option(b).sequence)
    p(Alternative[List].unite(List(Option(1), None, Option(2))), MonoidK[List].empty[Int], SemigroupK[Option].combineK(None, Option(2)), Bifunctor[Either].bimap(Left(1): Either[Int, String])(_ + 1, _.length), Bifunctor[Tuple2].leftMap((1, "a"))(_ * 2))
    p(FlatMap[Option].flatten(Option(Option(1))), FlatMap[List].ifM(List(true, false))(List(1), List(0)), Monad[Option].whileM_(Option(false))(Option(())), Defer[Eval].defer(Eval.now(1)).value, Comonad[NonEmptyList].extract(NonEmptyList.of(1, 2)), CoflatMap[Option].coflatMap(Option(1))(_.isDefined))
    p(ApplicativeError[[X] =>> Either[String, X], String].raiseError[Int]("e"), MonadError[Option, Unit].handleErrorWith(None: Option[Int])(_ => Some(0)), ApplicativeError[[X] =>> Either[String, X], String].fromOption(Option.empty[Int], "none"), (Left("x"): Either[String, Int]).handleError(_ => 0), (Left("x"): Either[String, Int]).attempt)
    p(Reducible[NonEmptyList].reduceMap(NonEmptyList.of(1, 2))(_.toString), UnorderedFoldable[List].size(List(1, 2)), Align[List].zipAll(List(1), List("a", "b"), 0, "z"), Semigroupal[Option].product(Option(1), Option("a")))
