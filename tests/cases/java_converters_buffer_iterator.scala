// The deprecated `JavaConverters` a library still calls: a Java list as a Scala buffer, which the
// list backs (an append on either side is seen on the other), and a Scala iterator as a Java one.
import scala.collection.JavaConverters.*

@main def run =
  val names = new java.util.ArrayList[String]()
  names.add("ada")
  names.add("grace")
  val buffer = names.asScala
  println(buffer.mkString(", "))
  println(buffer.length)
  val it = List(3, 4, 5).iterator.asJava
  var sum = 0
  while it.hasNext do sum += it.next()
  println(sum)
  val backed = new java.util.ArrayList[String]()
  backed.add("a")
  val view = backed.asScala
  view += "b"
  backed.add("c")
  println(s"${backed.size()} ${view.mkString(",")}")
  view.remove(0)
  println(backed)
