// teq: --werror
// The cases a sequence extractor's patterns leave, shown as dotty shows an `unapplySeq`'s product:
// its prefix and the elements of the sequence space (`projectSeq`), as scalac 3.8.4 reports them;
// a match over the lean std's `Vector` itself is checked as scalac checks scala-library's, sealed
// there, and so is one over a concrete sealed class, whose own instances its subclass leaves.
def a(x: Option[Vector[Boolean]]): Int = x match { case Some(Vector(true, _)) => 1; case None => 0 }
def e(x: Either[Vector[Int], Int]): Int = x match { case Left(Vector(a, b)) => a; case Right(n) => n }
def f(x: Option[Array[Int]]): Int = x match { case Some(Array()) => 0; case None => 1 }
def v(x: Vector[Boolean]): Int = x match { case Vector(true, _) => 1 }
sealed class Shape(val n: Int)
final class Square(n: Int) extends Shape(n)
def s(x: Shape): Int = x match { case q: Square => q.n }
// expect: 6:42: warning: match may not be exhaustive; missing: Some(Vector(false, _*)), Some(Vector(_, _, _, _*)), Some(Vector(_)), Some(Vector())
// expect: 7:43: warning: match may not be exhaustive; missing: Left(Vector(_, _, _, _*)), Left(Vector(_)), Left(Vector())
// expect: 8:37: warning: match may not be exhaustive; missing: Some(Array(_, _*))
// expect: 9:34: warning: match may not be exhaustive; missing: Vector(false, _*), Vector(_, _, _, _*), Vector(_), Vector()
// expect: 12:24: warning: match may not be exhaustive; missing: _: Shape
