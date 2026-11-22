// A kept body's object whose CLASS parent's constructor call is pickled as a block (the written named argument
// hoisted, then the constructor's default getters: utest's PPrinter.BlackWhite), which the products' reader does not
// read back: the object is built as before, the call left out (the block is not read back yet),
// where a trait parent's such call is refused. The written argument is the parameter's default, so the value is
// scalac's either way.
package kcpb
object Attrs { val Green: String = "green"; val Yellow: String = "yellow" }
case class P(width: Int = 100, color: String = Attrs.Green, prefix: String = Attrs.Yellow) {
  def show = s"$width $color $prefix"
}
object P {
  object Same extends P(color = Attrs.Green)
}
