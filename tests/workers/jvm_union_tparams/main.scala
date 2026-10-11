// A member's union over its own type parameter: the signature is completed on a worker, whose
// type parameter it names, and published into the base, which the JVM backend scans for the
// classes a union's part erases through.
class U0:
  def name: String = "u0"
class UA extends U0:
  override def name: String = "ua"
class UB extends U0

object Api:
  def u9[T <: UA](x: T | UB): String = x.name

@main def main(): Unit = println(Api.u9(new UA) + " " + Api.u9(new UB))
