final case class Request(keyOne: String)
@main def run(): Unit = println(make[Request]("abc"))
