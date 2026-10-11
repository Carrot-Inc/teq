// A program's ClassTag: its runtimeClass, unapply, newArray and wrap reached by dispatch through the
// trait, from a type pattern, a generic array and the program's own evidence parameters, each
// effect once; the trait's own unapply, equals, hashCode, toString and wrap read runtimeClass as
// scala-library's do, once each; the tags the compiler supplies are scala-library's, a primitive's
// named by its type and equal to itself, any other equal by class.
import scala.reflect.ClassTag

var calls = 0
class Upper extends ClassTag[String]:
  def runtimeClass: Class[?] = { calls += 1; classOf[String] }
  override def unapply(x: Any): Option[String] = { println(s"unapply $x"); x match
    case s: String if s.nonEmpty && s.head.isUpper => Some(s)
    case _ => None }
  override def newArray(n: Int): Array[String] = { println(s"newArray $n"); Array.fill(n)("new") }
  override def toString: String = "Upper"

var reads = 0
class Counted[T](c: Class[?]) extends ClassTag[T]:
  def runtimeClass: Class[?] = { reads += 1; c }

def make[T](n: Int)(using t: ClassTag[T]): Array[T] = new Array[T](n)
def firstOf[T](xs: List[Any])(using ClassTag[T]): Option[T] = xs.collectFirst { case t: T => t }
def tagOf[T](using t: ClassTag[T]): ClassTag[T] = t
def counted(label: String)(f: => Any): Unit =
  reads = 0
  val r = f
  println(s"$label $r reads $reads")

@main def run(): Unit =
  val t: ClassTag[String] = new Upper
  println(t.runtimeClass.getName)
  println(t.unapply("Abc"))
  println(t.unapply("abc"))
  println(t.newArray(2).toList)
  println(make[String](1)(using t).toList)
  println(firstOf[String](List(1, "abc", "Xyz"))(using t))
  println(List[Any]("q", "Q").map { case t(s) => s"tag $s"; case other => s"plain $other" })
  println(tagOf[String](using t))
  println(List(tagOf[Int], tagOf[String], tagOf[Long], tagOf[Unit], tagOf[Any], tagOf[AnyVal]))
  println(tagOf[Int] == ClassTag.Int)
  println(tagOf[String] == ClassTag(classOf[String]))
  println(tagOf[String].hashCode == classOf[String].hashCode)
  println(ClassTag(classOf[Int]) eq ClassTag.Int)
  println(make[Int](2).toList)
  println(tagOf[Int].unapply(1))
  println(tagOf[String].unapply(1))
  println(calls)
  val userInt = new Counted[Int](classOf[Int])
  counted("unapply")(userInt.unapply(1))
  counted("toString")(userInt.toString)
  counted("equals")(userInt == new Counted[Int](classOf[Int]))
  counted("equals Int")(userInt == ClassTag.Int)
  counted("hashCode")(userInt.hashCode == classOf[Int].hashCode)
  counted("wrap")(userInt.wrap.runtimeClass.isArray)
  counted("string unapply")(new Counted[String](classOf[String]).unapply("s"))
