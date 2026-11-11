trait Witness {
  type Out
  val out: Out
}
class Holder[T <: Witness & Singleton](val value: T) {
  def read: value.Out = value.out
}
class Use[T <: Witness & Singleton](val value: T) {
  val holder = new Holder(value)
  val result: value.Out = holder.read
  val explicit = new Holder[value.type](value)
  val control: value.Out = explicit.read
}
object Chosen extends Witness {
  type Out = Int
  val out = 7
}
@main def runSingletonConstructorInference(): Unit = println(new Use(Chosen).result)
