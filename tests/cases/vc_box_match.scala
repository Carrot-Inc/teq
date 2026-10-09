// A match and a type test on a value holding a value class test its box's class; the binder is the
// underlying value again, and an extractor of the class's companion takes it.
class V(val x: Int) extends AnyVal
object V { def unapply(v: V): Some[Int] = Some(v.x) }
class S(val s: String) extends AnyVal
case class CV(x: Int) extends AnyVal
object Main {
  def m1(a: Any): String = a match {
    case i: Int => s"Int $i"; case v: V => s"V ${v.x}"; case c: CV => s"CV ${c.x}"; case s: S => s"S ${s.s}"
    case s: String => s"Str $s"; case _ => "other"
  }
  def main(args: Array[String]): Unit = {
    val v = new V(1)
    println(m1(v)); println(m1(1)); println(m1(CV(2))); println(m1(new S("q"))); println(m1("q"))
    println((v: Any) match { case V(x) => s"ext $x"; case _ => "no" })
    println((CV(3): Any) match { case CV(x) => s"cv $x"; case _ => "no" })
    println(CV(3) match { case CV(x) => s"cv $x" })
    println(v match { case w: V => w.x })
    println(v match { case V(x) if x > 0 => "pos"; case _ => "neg" })
    println((1: Any).isInstanceOf[V]); println((v: Any).isInstanceOf[V]); println((v: Any).isInstanceOf[Int])
    println((v: Any).asInstanceOf[V].x)
    println(List[Any](new V(1), 1, new S("a"), "a").map { case _: V => "V"; case _: Int => "I"; case _: S => "S"; case _: String => "Str"; case _ => "?" }.mkString)
    try (v: Any) match { case _: Int => println("int") } catch { case _: MatchError => println("MatchError") }
  }
}
