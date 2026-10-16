// A `clone` overridden in a diamond: `Root`'s of result `CharSequence`, `Left`'s and `Right`'s of `String`.
// A class mixing in both takes the forwarder of the winner's signature and a bridge of each overridden
// signature, the intermediate `CharSequence` and `Object`'s, as dotty's `Bridges` pairs them; without the
// `CharSequence` one the JVM finds two defaults of it (`IncompatibleClassChangeError`). Built whole with
// DiamondClient.scala and split over this file's products alike (tests/classfile.scala, DiamondJava.java;
// tests/modules/clonediamond).
package clonediamond

trait Root:
  override def clone(): CharSequence = "root"
trait Left extends Root:
  override def clone(): String = "left"
trait Right extends Root:
  override def clone(): String = "right"
