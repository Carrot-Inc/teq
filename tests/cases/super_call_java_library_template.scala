// `super.m(..)` to a Java library method the std models by a template calls the superclass's method, as scalac
// writes it (`invokespecial` on the class `super` names), in a class, an anonymous class and a lambda, through an
// abstract class in the middle and through a class that inherits the method; so do `super.toString`,
// `super.hashCode` and `super.equals` where no class behind defines them (`Object`'s).
abstract class Mid(s: java.io.OutputStream) extends java.io.FilterOutputStream(s)
class G(s: java.io.OutputStream) extends java.io.FilterOutputStream(s)

class Plus(s: java.io.OutputStream) extends java.io.FilterOutputStream(s):
  override def write(b: Int): Unit = super.write(b + 1)
  override def flush(): Unit = { write(70); super.flush() }

class InLambda(s: java.io.OutputStream) extends java.io.FilterOutputStream(s):
  override def write(b: Int): Unit = { val f = () => super.write(b + 2); f() }

class ThroughMid(s: java.io.OutputStream) extends Mid(s):
  override def write(b: Int): Unit = super.write(b + 3)

class ThroughG(s: java.io.OutputStream) extends G(s):
  override def write(b: Int): Unit = super.write(b + 4)

class PB:
  def m: Int = 1
class Str extends PB:
  override def toString(): String = "str:" + super.toString().takeWhile(_ != '@')
class Hs:
  override def hashCode(): Int = super.hashCode() - System.identityHashCode(this)
class Eq:
  override def equals(o: Any): Boolean = super.equals(o)

object Main:
  def bytes(use: java.io.OutputStream => java.io.OutputStream): List[Byte] =
    val sink = new java.io.ByteArrayOutputStream
    val out = use(sink)
    out.write(64)
    out.flush()
    sink.toByteArray.toList

  def main(args: Array[String]): Unit =
    println(bytes(new Plus(_)))
    println(bytes(s => new java.io.FilterOutputStream(s) { override def write(b: Int): Unit = super.write(b + 5) }))
    println(bytes(new InLambda(_)))
    println(bytes(new ThroughMid(_)))
    println(bytes(new ThroughG(_)))
    println(new Str().toString)
    println(new Hs().hashCode)
    val e = new Eq
    println(s"${e.equals(e)} ${e.equals(new Eq)}")
