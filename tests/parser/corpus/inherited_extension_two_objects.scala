// One extension a trait declares, reached through two objects deriving from it: the receiver
// `Svg.Tag[Html.Tag[Int]]` has both objects in its implicit scope, and each candidate is called on
// the object it was found through, so `Svg`'s applies (scalac: "circle", "circle/rect").
trait Kit[Top]:
  final opaque type Tag[+N] = String
  def make[N](name: String): Tag[N] = name
  extension [N](self: Tag[N])
    def name: String = self
    def join(other: String): String = s"$self/$other"
object Html extends Kit[Any]
object Svg extends Kit[Any]
@main def main(): Unit =
  val t = Svg.make[Html.Tag[Int]]("circle")
  println(t.name)
  println(t.join("rect"))
