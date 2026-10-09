import scala.collection.mutable.ArrayBuffer

package frp:
  class Signal(val v: Int):
    def doubled = v * 2
  object Signal:
    def apply(v: Int): Signal = new Signal(v)
  package inner:
    val base = Signal(21)
    def show = s"inner sees ${base.doubled}"
end frp

package other {
  import frp.*
  case class Wrapper(s: Signal) {
    def buf = ArrayBuffer(s.v)
  }
  object Consts { val K = 7 }
}

import frp.inner.show

object Test:
  def main(args: Array[String]): Unit =
    println(frp.Signal(2).doubled)
    println(show)
    println(other.Wrapper(frp.Signal(5)).buf)
    println(other.Consts.K)
