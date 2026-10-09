// Sequence patterns over List, Seq, Vector and Array, with a rest binder.
case class Wrap(xs: List[Int])
def seqPat(xs: List[Int]): String = xs match
  case List() => "empty"
  case List(h, _*) if h < 0 => "negative head " + h
  case List(a) => "one " + a
  case List(a, b) => "two " + a + b
  case List(h, rest*) => "head " + h + " rest " + rest
def seqPat2(xs: Seq[Int]): String = xs match
  case Seq(1, rest*) => "starts1 " + rest
  case Seq() => "empty"
  case xs @ Seq(_, _) => "pair " + xs
  case _ => "?"
def arr(a: Array[String]): String = a match
  case Array(k, v) => k + "=" + v
  case Array(k) => k + " alone"
  case Array(k, more*) => k + " and " + more.mkString(",")
  case _ => "none"
def any(x: Any): String = x match
  case List(1, 2) | List(3) => "one two or three"
  case Vector(a, b) => "vec " + a + b
  case List(Some(v), _*) => "some first " + v
  case Seq(s: String, _*) => "string seq " + s
  case _ => "other"
def nested(w: Wrap): Int = w match
  case Wrap(List(a, b)) => a + b
  case Wrap(List(a, _*)) => a
  case Wrap(_) => 0
def old(xs: List[Int]): Int = xs match
  case List(a, tail @ _*) => a + tail.length
  case _ => -1
@main def main(): Unit =
  println(List(Nil, List(1), List(1, 2), List(1, 2, 3), List(-1, 2)).map(seqPat))
  println(seqPat2(Vector(1, 2, 3)) + " " + seqPat2(List()) + " " + seqPat2(List(2, 3)) + " " + seqPat2(List(2)))
  println(arr("a=b".split("=")) + " " + arr(Array("k")) + " " + arr(Array("k", "v", "w")) + " " + arr(Array()))
  println(List[Any](List(1, 2), List(3), Vector(4, 5), List(Some(6), None), Vector("s", 1), List(7, 8, 9), 3, Nil).map(any))
  println(nested(Wrap(List(1, 2))) + " " + nested(Wrap(List(5))) + " " + nested(Wrap(Nil)))
  println(old(List(1, 2, 3)) + " " + old(Nil))
  val List(p, q) = List(1, 2): @unchecked
  println(p + q)
  val Seq(first, others*) = Vector(1, 2, 3): @unchecked
  println(first + " " + others)
