// What a JDK class the standard library stands in for has beside its methods: its static nested
// classes (`AbstractMap.SimpleEntry`, the enum `Thread.State`), its interfaces' default methods
// on a program's class (`Predicate.negate`, `Iterator.remove`, a `hasNext()` written with its
// parentheses), and a generic argument whose type arguments the parameter's wildcard leaves to
// the enclosing call (`collect(toList())`).
import java.util.AbstractMap
import java.util.function.Predicate

class Empty extends Predicate[String]:
  def test(s: String): Boolean = s.isEmpty

class Once extends java.util.Iterator[String]:
  private var done = false
  def hasNext(): Boolean = !done
  def next(): String = { done = true; "x" }

class Collector[T, A, R]
class Source[T]:
  def collect[A, R](c: Collector[? >: T, A, R]): R = null.asInstanceOf[R]
def toList[T](): Collector[T, ?, java.util.List[T]] = new Collector[T, Object, java.util.List[T]]

object Main:
  def main(args: Array[String]): Unit =
    val e: java.util.Map.Entry[String, Int] = new AbstractMap.SimpleEntry[String, Int]("x", 1)
    println(e.getKey())
    println(e.setValue(2))
    println(e)
    println(e == new AbstractMap.SimpleImmutableEntry("x", 2))
    println(new AbstractMap.SimpleEntry[String, Int](e).getValue)
    println(Thread.State.NEW.name())
    println(Thread.State.valueOf("BLOCKED").ordinal())
    println(Thread.State.values().length)
    println(Thread.currentThread().getState())
    println(new Empty().negate().test("x"))
    println(new Empty().and(s => s.length < 3).test(""))
    println(new Empty().or(s => s == "y").test("y"))
    println(Predicate.not[String](new Empty()).test(""))
    println(Predicate.isEqual[String]("a").test("a"))
    val it = new Once
    println(it.hasNext())
    println(it.next())
    try it.remove()
    catch case ex: UnsupportedOperationException => println(ex.getMessage)
    val r: java.util.List[String] = new Source[String]().collect(toList())
    println(r)
    println(java.util.stream.Stream.of("a", "b").collect(java.util.stream.Collectors.toList()))
