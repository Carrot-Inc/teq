//> using dep com.lihaoyi::sourcecode:0.4.2
// jars: sourcecode
package demo.names

def site(tag: String)(using n: sourcecode.Name, f: sourcecode.FullName, l: sourcecode.Line): String =
  s"$tag: ${n.value} | ${f.value} | ${l.value}"
def full(using f: sourcecode.FullName): String = f.value

// A value definition of a for comprehension is a val of that name; a pattern is not.
def forVals: List[String] =
  for
    x <- List(1, 2)
    plain = site("plain" + x)
    typed: String = site("typed" + x)
    (first, second) = (site("pattern" + x), x)
  yield plain + "\n" + typed + "\n" + first + "\n" + site("yield" + x)

def forValThenGuard: List[String] =
  for
    x <- List(1, 2, 3)
    beforeGuard = site("beforeGuard" + x)
    if x > 2
    afterGuard = site("afterGuard" + x)
  yield beforeGuard + "\n" + afterGuard

def forDo: String =
  var out = ""
  for
    x <- List(1)
    inDo = site("inDo")
  do out = out + inDo + "\n" + site("doBody")
  out

class Forms:
  def nested: List[String] =
    for
      x <- List(1)
      y <- List(2)
      sum = site("sum" + (x + y))
    yield sum

// Anonymous givens are named after the type and the heads of its top-level arguments only.
trait Show[A]:
  def show(a: A): String

case class Box[A](a: A)

given Show[Int] with
  def show(a: Int): String = full
given [A](using s: Show[A]): Show[List[A]] with
  def show(a: List[A]): String = full
given Show[Box[Box[Int]]] with
  def show(a: Box[Box[Int]]): String = full
given Show[(Int, String)] with
  def show(a: (Int, String)): String = full
given Show[Int => String] with
  def show(a: Int => String): String = full
given Show[Map[String, List[Int]]] with
  def show(a: Map[String, List[Int]]): String = full
given Show[Int | String] with
  def show(a: Int | String): String = full
given (Int => Box[Int]) = x => Box(full.length + x)
given demo.names.Box[String] = Box(full)
given Either[String, Box[Int]] = Left(full)

object Scope:
  given demo.names.Box[List[Int]] = Box(List(full.length))

@main def run(): Unit =
  forVals.foreach(println)
  forValThenGuard.foreach(println)
  println(forDo)
  Forms().nested.foreach(println)
  println(summon[Show[Int]].show(1))
  println(summon[Show[List[Int]]].show(Nil))
  println(summon[Show[Box[Box[Int]]]].show(Box(Box(1))))
  println(summon[Show[(Int, String)]].show((1, "")))
  println(summon[Show[Int => String]].show(_.toString))
  println(summon[Show[Map[String, List[Int]]]].show(Map.empty))
  println(summon[Show[Int | String]].show(1))
  println(summon[Int => Box[Int]](1))
  println(summon[Box[String]])
  println(summon[Either[String, Box[Int]]])
  println(given_Box_String == summon[Box[String]])
  println(Scope.given_Box_List)
