// A `clone` overridden in a diamond, `Root`'s of result `CharSequence` and `Left`'s and `Right`'s of
// `String`: the downstream class mixing in both takes the forwarder of the winner's signature and a
// bridge of each overridden one (dotty's `Bridges`), the same bytes whether it reads these traits from
// their sources or from these products. tests/classfile.scala runs a Java client over both builds.
package clonediamond

trait Root:
  override def clone(): CharSequence = "root"
trait Left extends Root:
  override def clone(): String = "left"
trait Right extends Root:
  override def clone(): String = "right"
