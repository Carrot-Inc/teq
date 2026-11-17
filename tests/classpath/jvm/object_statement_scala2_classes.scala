// jars: scala-library kantan-codecs
// std: scala-library
//> using dep com.nrinaudo:kantan.codecs_2.13:0.6.0
// An object that extends a class or mixes in a trait of a Scala 2 library, named as a block's statement: a
// Scala 2 class always runs an initialiser to the reference, and so does a Scala 2 trait that holds a concrete
// member; a Scala 2 trait of abstract members alone runs none. Each object's effect sits where the reference
// sees no initialiser of the object's own (a mixed-in trait's parameter default, a secondary constructor's
// argument).
import kantan.codecs.resource.{Closeable, ResourceError}

trait Noted(name: String = { println("  made: a Scala 2 class's object"); "x" })
object Failure extends kantan.codecs.error.Error("failed") with Noted {
  def canEqual(that: Any): Boolean = false
  def productArity: Int = 0
  def productElement(n: Int): Any = null
}

class Carrier1(x: Int) { def this() = this({ println("  made: a Scala 2 trait with a method"); 1 }) }
object Encoders extends Carrier1() with kantan.codecs.EncoderCompanion[String, Any]

class Carrier2(x: Int) { def this() = this({ println("  made: a Scala 2 trait of abstract members"); 1 }) }
object Closer extends Carrier2() with Closeable[String] {
  def close(a: String): Either[ResourceError.CloseError, Unit] = Right(())
}

object Main {
  @annotation.nowarn("msg=pure expression")
  def main(args: Array[String]): Unit = {
    println("a Scala 2 class"); Failure
    println("a Scala 2 trait with a method"); Encoders
    println("a Scala 2 trait of abstract members"); Closer
    println("end")
  }
}
