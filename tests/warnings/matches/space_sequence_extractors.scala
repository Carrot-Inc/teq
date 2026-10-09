// Every sequence extractor's elements as dotty's `projectSeq` makes them, products of `::` ending
// in `Nil` or the list: the pattern's space where its type derives from `List`, the one component
// of the extractor's product otherwise (`project` of an `unapplySeq`, `signature`'s `List[elem]`):
// a `Vector`, a `Seq`, an `Array`, inside a checked type.
def vectorSome(x: Option[Vector[Boolean]]): Int = x match { case Some(Vector(true, _)) => 1; case None => 0 }
def seqCovered(x: Option[Seq[Int]]): Int = x match { case Some(Seq()) => 0; case Some(Seq(_, _*)) => 1; case None => 2 }
def vectorCovered(x: Option[Vector[Int]]): Int = x match { case Some(Vector()) => 0; case Some(Vector(_, _*)) => 1; case None => 2 }
def listAny(x: Option[List[Boolean]]): Int = x match { case Some(List(true)) => 1; case Some(_) => 2; case None => 3 }
def vectorLeft(x: Either[Vector[Int], Int]): Int = x match { case Left(Vector(a, b)) => a; case Right(n) => n }
def arrayEmpty(x: Option[Array[Int]]): Int = x match { case Some(Array()) => 0; case None => 1 }
def vectorAfter(x: Option[Vector[Int]]): Int = x match
  case Some(Vector(_, _*)) => 1
  case Some(_) => 2
  case Some(Vector()) => 3
  case None => 0

@main def run(): Unit =
  println(vectorSome(None) + seqCovered(Some(Seq(1))) + vectorCovered(None) + listAny(None) + vectorLeft(Right(1)) + arrayEmpty(None) + vectorAfter(None))
