def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

def parse(s: String): Either[String, Int] = s.toIntOption.toRight("bad: " + s)

@main def main(): Unit =
  val some: Option[Int] = Some(3)
  val none: Option[Int] = None
  show("toRight", some.toRight("e"), none.toRight("e"), some.toLeft("r"), none.toLeft("r"))
  show("zip", some.zip(Some("a")), some.zip(none), none.zip(some))
  show("unzip", Option((1, "a")).unzip, Option.empty[(Int, String)].unzip)
  show("flatten", Some(Some(1)).flatten, Some(None).flatten, Option.empty[Option[Int]].flatten)
  show("filterNot", some.filterNot(_ > 2), some.filterNot(_ > 5))
  show("when", Option.when(true)(1), Option.when(false)(1), Option.unless(true)(1), Option.unless(false)(1))
  show("iterator", some.iterator.toList, none.iterator.toList, some.toSeq, some.toSet, some.toVector, none.size)
  show("in collections", List(1, 2) ++ some, List(some, none, Some(5)).flatten, List(1, 2, 3).flatMap(x => Option.when(x != 2)(x)))
  show("vector of options", Vector(some, none).flatten, Set(1, 2).flatMap(x => some.map(_ + x)))
  show("for mixed",
    for
      x <- List(1, 2, 3)
      y <- Option.when(x % 2 == 1)(x * 10)
    yield y)
  show("map2", for a <- some; b <- Some(4) yield a + b)
  show("fold", some.fold("none")(_.toString), none.fold("none")(_.toString))
  show("contains", some.contains(3), some.contains(4), none.contains(3), some.exists(_ > 1), none.forall(_ > 1))
  show("orElse", none.orElse(some), some.orElse(Some(0)), none.getOrElse(7), some.get, some.nonEmpty, none.isDefined)
  val text: String = "x"
  show("apply", Option(text), Option(1), Option(List(1)))

  val right: Either[String, Int] = Right(4)
  val left: Either[String, Int] = Left("no")
  show("swap", right.swap, left.swap)
  show("merge", parse("12").map(_.toString).merge, parse("zz").map(_.toString).merge)
  show("left", right.left.toOption, left.left.toOption, left.left.map(_.length), right.left.map(_.length), left.left.toSeq)
  show("left more", left.left.getOrElse("d"), right.left.getOrElse("d"), left.left.exists(_ == "no"), left.left.forall(_ == "x"))
  show("exists", right.exists(_ > 3), left.exists(_ > 3), right.forall(_ > 5), left.forall(_ > 5), right.contains(4), left.contains(4))
  show("orElse", left.orElse(Right(0)), right.orElse(Right(0)), left.getOrElse(9), right.getOrElse(9))
  show("filterOrElse", right.filterOrElse(_ > 5, "small"), right.filterOrElse(_ > 1, "small"), left.filterOrElse(_ > 5, "small"))
  show("toSeq", right.toSeq, left.toSeq, right.toOption, left.toOption, right.isRight, left.isLeft)
  show("cond", Either.cond(true, 1, "l"), Either.cond(false, 1, "l"))
  show("flatten", Right[String, Either[String, Int]](Right(1)).flatten, Right[String, Either[String, Int]](Left("in")).flatten)
  show("fold", right.fold(_.length, _ * 2), left.fold(_.length, _ * 2))
  show("for",
    for
      a <- parse("1")
      b <- parse("2")
    yield a + b,
    for
      a <- parse("1")
      b <- parse("x")
    yield a + b)
  show("partitionMap", List("1", "a", "2").partitionMap(parse))
  show("option find", some.find(_ > 1), some.find(_ > 5), none.find(_ > 1), some.count(_ > 1), none.count(_ > 1))
  show("option fold", some.foldLeft(1)(_ + _), none.foldLeft(1)(_ + _), some.head, some.headOption, none.lastOption)
