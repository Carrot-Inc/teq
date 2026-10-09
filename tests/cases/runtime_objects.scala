// The objects of `scala.runtime` that compiled library bodies call: `ScalaRunTime._hashCode` of
// a case class's `hashCode`, `Scala3RunTime.assertFailed` of an inlined `assert`, and
// `BoxesRunTime.equalsNumObject` of boxed numeric equality.
import scala.runtime.{BoxesRunTime, ScalaRunTime, Scala3RunTime}

final case class Key(name: String, n: Int)

object Main:
  def main(args: Array[String]): Unit =
    val k = Key("a", 1)
    println(ScalaRunTime._hashCode(k) == k.hashCode)
    println(ScalaRunTime._toString(k))
    println(BoxesRunTime.equalsNumObject(java.lang.Integer.valueOf(3), java.lang.Long.valueOf(3L)))
    println(BoxesRunTime.equalsNumObject(java.lang.Integer.valueOf(3), "3"))
    println(BoxesRunTime.equals(java.lang.Double.valueOf(2.0), java.lang.Integer.valueOf(2)))
    try Scala3RunTime.assertFailed("broken")
    catch case e: AssertionError => println(e.getMessage)
    try Scala3RunTime.assertFailed()
    catch case e: AssertionError => println(e.getMessage)
