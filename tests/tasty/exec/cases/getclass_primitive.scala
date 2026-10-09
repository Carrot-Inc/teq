// A primitive's getClass is its own class, `Class[Int]`, as scalac's `ConstFold` has it, where a
// reference's is `Class[? <: T]`. teq's own builds give the box's class (false); scalac's
// regeneration from teq's TASTy prints scalac's true.
object GetclassPrimitive {
  def exact(i: Int): Class[Int] = i.getClass
  def main(args: Array[String]): Unit =
    println(exact(1) == classOf[Int])
}
