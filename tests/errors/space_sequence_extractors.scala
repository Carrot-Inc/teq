// teq: --werror
// The cases a sequence extractor's patterns leave, shown as dotty shows an `unapplySeq`'s product:
// its prefix and the elements of the sequence space (`projectSeq`), as scalac 3.8.4 reports them.
def a(x: Option[Vector[Boolean]]): Int = x match { case Some(Vector(true, _)) => 1; case None => 0 }
def e(x: Either[Vector[Int], Int]): Int = x match { case Left(Vector(a, b)) => a; case Right(n) => n }
def f(x: Option[Array[Int]]): Int = x match { case Some(Array()) => 0; case None => 1 }
// expect: 4:42: warning: match may not be exhaustive; missing: Some(Vector(false, _*)), Some(Vector(_, _, _, _*)), Some(Vector(_)), Some(Vector())
// expect: 5:43: warning: match may not be exhaustive; missing: Left(Vector(_, _, _, _*)), Left(Vector(_)), Left(Vector())
// expect: 6:37: warning: match may not be exhaustive; missing: Some(Array(_, _*))
