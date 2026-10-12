case class Config(id: String)
given Loader[Config] = AutoLoader.loader

@main def run(): Unit =
  try summon[Loader[Config]].load("x")
  catch case e: Exception => println(e.getMessage)
