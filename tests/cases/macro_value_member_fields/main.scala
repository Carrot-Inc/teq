final class Env(size: Int):
  lazy val repo: Long = 2L
  val count: Int = 3

class Api(env: Env):
  def show: String = Probe.fields(env)

@main def run(): Unit = println(Api(Env(1)).show)
