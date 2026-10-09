// A right-associative extension method takes the left operand as its leading parameter and is
// found through the right operand.
class Vec[A](val xs: List[A]):
  override def toString = "Vec(" + xs.mkString(",") + ")"

object Vec:
  extension [A](x: A) def +:(v: Vec[A]): Vec[A] = Vec(x :: v.xs)

extension [A](v: Vec[A])
  def ++:(other: Vec[A]): Vec[A] = Vec(other.xs ++ v.xs)
  def :::(prefix: Vec[A]): Vec[A] = Vec(prefix.xs ++ v.xs)
  def append(other: Vec[A]): Vec[A] = Vec(v.xs ++ other.xs)

extension (s: String)
  def /:(v: Vec[Int]): String = s + "/" + v.xs.sum

def t[A](label: String, a: A): A =
  println("  " + label)
  a

@main def run(): Unit =
  val v = Vec(List(1, 2))
  println(Vec(List(9)) ++: v)
  println(v.append(Vec(List(9))))
  println(0 +: v)
  println(Vec(List(9)) ::: v)
  println("sum" /: v)
  println(t("l", 7) +: t("r", Vec(List(8))))
  println(0 +: 1 +: v)
