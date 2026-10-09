// Default methods of the `java.util` interfaces and the converters to them, as zio's bodies call
// them: `Collection.removeIf`, `Iterable.forEach`, `CollectionConverters`' `asJava` and
// `asJavaCollection`, `Iterable.sizeCompare` and `Iterable.fill`.
import java.util.function.{Consumer, Predicate}
import scala.jdk.CollectionConverters.*

object Main:
  def main(args: Array[String]): Unit =
    val set = new java.util.HashSet[Int]()
    (1 to 6).foreach(set.add(_))
    println(set.removeIf(new Predicate[Int] { def test(x: Int): Boolean = x % 2 == 0 }))
    println(set.removeIf(_ > 100))
    var total = 0
    set.forEach(new Consumer[Int] { def accept(x: Int): Unit = total += x })
    println(total)
    val list = new java.util.ArrayList[String]()
    list.add("x")
    list.add("y")
    val seen = new StringBuilder
    list.forEach(s => seen.append(s))
    println(seen)
    val jc: java.util.Collection[Int] = List(3, 1, 2).asJavaCollection
    println(jc.size())
    println(jc.contains(1))
    val jl: java.util.List[Int] = Vector(7, 8).asJava
    println(jl.get(1))
    val back = jl.asScala
    println(back.sum)
    val it: Iterable[Int] = List(1, 2, 3)
    println(s"${it.sizeCompare(2)} ${it.sizeCompare(3)} ${it.sizeCompare(4)}")
    println(Iterable.fill(3)("z").toList)
