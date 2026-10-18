// A `scala.collection.Seq` extractor binds its elements at the scrutinee's element type, as dotty's
// `unapplySeqTypeElemTp` reads it off the sequence's `apply`; `Array.concat`; the decorators of the
// deprecated `scala.collection.JavaConverters` that libraries compiled against it still call; and a
// `ConcurrentHashMap` seen as a `ConcurrentMap`.
import scala.collection.JavaConverters.*
import java.util.concurrent.{ConcurrentHashMap, ConcurrentMap}

object Main:
  def one[O](o: O): List[O] = List(o)

  def emits[O](os: scala.collection.Seq[O]): List[O] = os match
    case Nil => Nil
    case scala.collection.Seq(x) => one(x)
    case scala.collection.Seq(x, rest*) => x :: rest.reverse.toList
    case _ => os.toList

  def main(args: Array[String]): Unit =
    println(emits(Vector(1)))
    println(emits(scala.collection.mutable.ArrayBuffer("a", "b", "c")))
    println(emits(Nil))
    val joined = Array.concat(Array(1, 2), Array.empty[Int], Array(3))
    println(joined.mkString(",") + " " + joined.length)
    val bytes: Array[Byte] = Array.concat("ab".getBytes, "c".getBytes)
    println(new String(bytes))
    println(Array.concat[String]().length)
    val javaSet = Set("x", "y").asJava
    println(javaSet.size + " " + javaSet.contains("x") + " " + javaSet.contains("z") + " " + javaSet.contains(1))
    val mutableSet = scala.collection.mutable.LinkedHashSet(3, 1, 2)
    val view = mutableSet.asJava
    mutableSet += 4
    println(view.size + " " + view.contains(4))
    val javaMap = Map("k" -> 1).asJava
    println(javaMap.get("k") + " " + javaMap.size)
    val iterable = List(1, 2, 3).asJava
    println(iterable.size + " " + iterable.get(2))
    val list = new java.util.ArrayList[String]()
    list.add("p")
    list.add("q")
    println(list.asScala.mkString("+") + " " + (list: java.util.Collection[String]).asScala.size)
    val back = new java.util.HashMap[String, Int]()
    back.put("a", 1)
    println(back.asScala.toList)
    val m: ConcurrentMap[String, String] = new ConcurrentHashMap[String, String]()
    println(s"${m.putIfAbsent("a", "1")} ${m.putIfAbsent("a", "2")} ${m.get("a")} ${m.replace("a", "1", "3")} ${m.get("a")}")
