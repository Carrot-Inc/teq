object Use:
  val force: O.type = prefix
  import prefix.*
  val result = x
@main def run(): Unit = println(Use.result)
