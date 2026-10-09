//> using platform js
// Type arguments an argument leaves open follow the bounds of the open type variable that is
// expected of it: `getOrElse[B >: A](default: => B)`, and the join of if/else and match branches.
final case class Config(values: Map[String, Int])

def lookup(key: String): Option[Map[Int, String]] =
  if key == "a" then Some(Map(1 -> "one")) else None

def merge(base: Map[Int, String], extra: Option[Map[Int, String]]): Map[Int, String] =
  base ++ extra.getOrElse(Map.empty)

def sizes(c: Boolean): Map[Int, String] =
  val m = if c then Map(2 -> "two") else Map.empty
  m

def pick(n: Int): Set[Int] =
  n match
    case 0 => Set.empty
    case 1 => Set(1)
    case _ => Set(1, 2)

@main def main(): Unit =
  val a = Option(Map(1 -> "a")).getOrElse(Map.empty)
  println(a.get(1).map(_.length))
  val none: Option[List[Int]] = None
  println(none.getOrElse(Nil).map(_ + 1))
  println(Option(Set(1)).getOrElse(Set.empty) + 2)
  println(lookup("a").getOrElse(Map.empty).get(1).map(_.toUpperCase))
  println(lookup("b").getOrElse(Map.empty).size)
  val f: Map[Int, String] = lookup("b").getOrElse(Map.empty)
  println(f)
  println(merge(Map(0 -> "zero"), lookup("a")))
  println(sizes(true).get(2))
  println(sizes(false).size)
  println(pick(0).size + pick(1).size + pick(2).size)
  val nested = if a.isEmpty then Map.empty else if a.size > 3 then Map(9 -> "nine") else a
  println(nested.get(1))
  val fromMatch = a.get(1) match
    case Some(s) => Map(s -> 1)
    case None => Map.empty
  println(fromMatch.get("a"))
  val vec = Option(Vector(1.5)).getOrElse(Vector.empty)
  println(vec.map(_ * 2).sum)
  val cfg = Config(Map("x" -> 1))
  println(Option(cfg).map(_.values).getOrElse(Map.empty).get("x"))
  val lambda = (c: Boolean) => if c then List(1) else List.empty
  println(lambda(false).length + lambda(true).length)
