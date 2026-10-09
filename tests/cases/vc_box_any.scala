// A value class where a reference is wanted is its box on the JVM: its toString, getClass, type tests and
// equality are the box's (the dotty audit's probe e10_vc_box).
class V(val x: Int) extends AnyVal
class S(val s: String) extends AnyVal
case class CV(x: Int) extends AnyVal
object Main {
  def id[T](t: T): T = t
  def main(args: Array[String]): Unit =
    println(new V(1).toString); println((new V(1): Any).toString); println(id(new V(1)).toString)
    println((new V(1): Any).getClass.getName); println(new V(1).getClass.getName); println(id(new V(1)).getClass.getName)
    println((new V(1): Any) match { case i: Int => s"Int $i"; case v: V => s"V ${v.x}"; case _ => "other" })
    println((new S("a"): Any) match { case s: String => s"String $s"; case v: S => s"S ${v.s}"; case _ => "other" })
    println(List[Any](new V(1), 1, new S("a"), "a").map { case _: V => "V"; case _: Int => "I"; case _: S => "S"; case _: String => "Str"; case _ => "?" }.mkString)
    println(CV(1)); println(CV(1) == CV(1)); println((CV(1): Any) == 1); println(Set[Any](CV(1), 1).size)
    println(new V(1).hashCode); println((new V(1): Any).hashCode); println(new S("a").hashCode == "a".hashCode)
    println(new V(1).isInstanceOf[V]); println((1: Any).isInstanceOf[V]); println(("a": Any).isInstanceOf[S])
}
