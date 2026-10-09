// A written argument that branches after a defaulted parameter: the default's placeholder slot
// must hold a value of the parameter's type for the verifier (a null, not a boxed unit).
case class Field(name: String, kind: Int, description: Option[String] = None, enumValues: Option[List[String]] = None)

object Main:
  def mk(flag: Boolean): Field =
    Field(name = "x", kind = 1, enumValues = Some(if flag then List("a", "b") else List("a")))

  def main(args: Array[String]): Unit =
    println(mk(true))
    println(mk(false))
