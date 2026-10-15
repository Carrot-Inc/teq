import scala.util.DynamicVariable

// scala-library's `DynamicVariable`, which os-lib's paths read their serializer from: a value bound
// for the extent of a thunk and restored after it, a failure's included.
@main def run(): Unit =
  val depth = DynamicVariable(0)
  def show(): Unit = println(s"depth ${depth.value}")
  show()
  depth.withValue(1) {
    show()
    depth.withValue(depth.value + 1)(show())
    show()
  }
  show()
  val failed = scala.util.Try(depth.withValue(9)(throw RuntimeException("inside"))).isFailure
  println(s"$failed ${depth.value}")
  depth.value = 3
  println(depth)
  val result: String = depth.withValue(5)(s"result at ${depth.value}")
  println(result)
