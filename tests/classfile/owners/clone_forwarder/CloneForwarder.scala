// A trait's override of `clone` with a narrower result, mixed in through a trait extending it: the class
// takes the forwarder of the overriding method's signature, `clone(): String`, and a bridge of `Object`'s
// to it, as dotty's `Mixin.mixinForwarders` and `Bridges` write them, which a Java client compiled
// against the class's products calls by either (JavaClient.java, ReflectClient.java; tests/classfile.scala).
package cloneforwarder

trait C:
  override def clone(): String = "clone"
trait D extends C
class E extends D

object Main:
  def c(x: D): String = x.clone()
  def main(args: Array[String]): Unit =
    println(c(new E))
