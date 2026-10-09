// jars: scala-library
// scala-library's value classes are boxes of their own where a reference is wanted, and the jar's generic code
// (an Ordering, a sort, a Map) takes a program's value class as its box.
class V(val x: Int) extends AnyVal
case class CV(x: Int) extends AnyVal
object Main {
  def show(a: Any): String = a match {
    case r: scala.runtime.RichInt => s"RichInt ${r.self}"
    case s: scala.collection.StringOps => s"StringOps ${s.size}"
    case v: V => s"V ${v.x}"
    case other => s"other ${other.getClass.getName}"
  }
  def main(args: Array[String]): Unit = {
    val r = new scala.runtime.RichInt(3)
    println((r: Any).getClass.getName); println(show(r)); println(r.max(5)); println((r: Any) == new scala.runtime.RichInt(3))
    val s = new scala.collection.StringOps("abc")
    println((s: Any).getClass.getName); println(show(s)); println(s.reverse); println((s: Any).hashCode == "abc".hashCode)
    val a = Predef.ArrowAssoc(1)
    println((a: Any).getClass.getName); println(a -> 2); println(show(3))
    val vs = List(new V(3), new V(1), new V(2))
    given Ordering[V] = Ordering.by(_.x)
    println(vs.sorted.map(_.x)); println(vs.max.x); println(vs.min.x); println(vs.sorted.map(show))
    println(Array(new V(5), new V(4)).sortBy(_.x).map(_.x).mkString(","))
    val m = Map(new V(2) -> "b", new V(1) -> "a")
    println(m.toSeq.sortBy(_._1).map(_._2).mkString); println(m.keySet.contains(new V(1)))
    println(List(CV(2), CV(1)).sortBy(_.x)); println(Set(CV(1), CV(1), CV(2)).size); println(Vector(new V(7)).map(show))
    println(Option(new V(8)).fold("none")(show)); println(Some(new V(9)).collect { case v if v.x > 5 => v.x })
  }
}
