// An assignment operator on a var of a type that has no such operator is the assignment, unless something the
// site sees provides the operator: a given's extension in a nested frame, a local import, a local conversion, an
// implicit class of a block, an extension of the enclosing class. The sites before and after each such scope
// are assignments again (a kept answer for the type, the operator and the file's context holds only where the
// site's own frames add nothing).
import scala.language.implicitConversions
trait Ops:
  extension (n: Int) def +=(k: Int): Unit
object Provider:
  extension (n: Int) def +=(k: Int): Unit = println("imported extension " + (n + k))
class Acc(val n: Int):
  def +=(k: Int): Unit = println("conversion " + (n + k))
class Host:
  extension (n: Int) def +=(k: Int): Unit = println("class extension " + (n + k))
  def run(): Unit =
    var x = 1
    x += 1
    println(x)
class Plain:
  def run(): Unit =
    var x = 1
    x += 1
    println(x)
object Test:
  def first(): Int = { var x = 1; x += 1; x }
  def givenInFrame(): Int =
    var x = 1
    x += 1
    {
      given Ops with
        extension (n: Int) def +=(k: Int): Unit = println("given extension " + (n + k))
      x += 1
    }
    x += 1
    x
  def localImport(): Int =
    var x = 1
    x += 1
    {
      import Provider.*
      x += 1
    }
    x += 1
    x
  def localConversion(): Int =
    var x = 1
    {
      given Conversion[Int, Acc] = n => new Acc(n)
      x += 1
    }
    x += 1
    x
  def implicitClassInBlock(): Int =
    var x = 1
    {
      implicit class More(val n: Int) { def +=(k: Int): Unit = println("implicit class " + (n + k)) }
      x += 1
    }
    x += 1
    x
  def last(): Int = { var x = 1; x += 1; x }
  def main(args: Array[String]): Unit =
    println(first())
    println(givenInFrame())
    println(localImport())
    println(localConversion())
    println(implicitClassInBlock())
    new Host().run()
    new Plain().run()
    println(last())
